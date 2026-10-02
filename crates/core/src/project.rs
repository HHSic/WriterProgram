//! Project folder (작품 폴더) and its structure.
//!
//! ```text
//! 작품 폴더/
//!   project.json        title, type, goals, parts and chapter order
//!   manuscript/<id>.md  chapters (회차·장), one file each
//!   planning/<id>.md    planning documents (기획)
//!   .snapshots/<id>/    records (기록) per document
//!   .trash/             deleted documents, kept 30 days
//! ```
//!
//! Parts (부) exist only in `project.json`: moving a chapter between parts
//! never moves its file, which keeps folder sync simple.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::cards::{self, CardSummary, CardType};
use crate::copies::{self, CopyInfo};
use crate::count::{Counts, count_blocks};
use crate::doc::{self, DocFile, DocMeta, Section};
use crate::format::{self, ManuscriptFormat};
use crate::layout::PageMetrics;
use crate::markup::Block;
use crate::store::{
    atomic_write, modified_iso, new_id, now_iso, read_text, rename_retry, safe_file_name,
};
use crate::{Error, Result, snapshot, trash};

pub const PROJECT_FILE: &str = "project.json";
pub const MANUSCRIPT_DIR: &str = "manuscript";
pub const PLANNING_DIR: &str = "planning";
pub const APP_NAME: &str = "WriterProgram";
pub const FORMAT_VERSION: u32 = 1;

/// Deleted documents are kept this many days.
pub const TRASH_DAYS: i64 = 30;
/// Automatic records older than this are removed. Records kept by hand stay.
pub const AUTO_RECORD_DAYS: i64 = 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectKind {
    Webnovel,
    Print,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Goal {
    /// Target length of one chapter.
    #[serde(default)]
    pub per_doc: Option<u32>,
    /// Whether targets count spaces (공백 포함).
    #[serde(default = "yes")]
    pub count_spaces: bool,
    #[serde(default)]
    pub daily: Option<u32>,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub docs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub app: String,
    pub format: u32,
    pub id: String,
    pub title: String,
    pub kind: ProjectKind,
    #[serde(default)]
    pub pen_name: String,
    #[serde(default)]
    pub created: String,
    pub goal: Goal,
    /// Symbol shown for scene breaks on screen and in exports.
    #[serde(default = "default_scene_break")]
    pub scene_break: String,
    /// Id or name of the manuscript format preset in use (원고 서식).
    #[serde(default)]
    pub preset: String,
    /// Manuscript format. Missing in projects made before it existed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub manuscript_format: Option<ManuscriptFormat>,
    /// Kinds of setting cards (인물, 장소, 용어 and user-made ones).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub card_types: Option<Vec<CardType>>,
    #[serde(default)]
    pub parts: Vec<Part>,
    #[serde(default)]
    pub planning: Vec<String>,
    /// Keys written by newer versions, kept as they are.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

fn default_scene_break() -> String {
    "◆".into()
}

impl Project {
    /// Kinds of setting cards; projects without any use the three defaults.
    pub fn card_types(&self) -> Vec<CardType> {
        self.card_types.clone().unwrap_or_else(cards::default_types)
    }

    /// The manuscript format in effect; older projects fall back to their preset.
    pub fn manuscript_format(&self) -> ManuscriptFormat {
        self.manuscript_format.clone().unwrap_or_else(|| {
            format::builtin(&self.preset).unwrap_or_else(|| format::default_for(self.kind))
        })
    }

    fn part_mut(&mut self, part_id: &str) -> Result<&mut Part> {
        self.parts
            .iter_mut()
            .find(|p| p.id == part_id)
            .ok_or_else(|| Error::NotFound("부를 찾을 수 없음".into()))
    }

    /// Removes a document from the structure and returns where it was.
    pub(crate) fn detach(&mut self, doc_id: &str) -> Option<(Option<String>, usize)> {
        for part in &mut self.parts {
            if let Some(i) = part.docs.iter().position(|d| d == doc_id) {
                part.docs.remove(i);
                return Some((Some(part.id.clone()), i));
            }
        }
        if let Some(i) = self.planning.iter().position(|d| d == doc_id) {
            self.planning.remove(i);
            return Some((None, i));
        }
        None
    }

    /// Puts a manuscript document back at `index` of `part_id`, or at the end
    /// of the last part when that part is gone.
    pub(crate) fn attach_manuscript(&mut self, doc_id: &str, part_id: Option<&str>, index: usize) {
        if self.parts.is_empty() {
            self.parts.push(new_part("1부"));
        }
        let part = match part_id.and_then(|id| self.parts.iter().position(|p| p.id == id)) {
            Some(i) => &mut self.parts[i],
            None => self.parts.last_mut().expect("at least one part"),
        };
        let index = index.min(part.docs.len());
        part.docs.insert(index, doc_id.to_string());
    }
}

fn new_part(title: &str) -> Part {
    Part {
        id: new_id(),
        title: title.into(),
        docs: Vec::new(),
    }
}

pub fn load(root: &Path) -> Result<Project> {
    let path = root.join(PROJECT_FILE);
    if !path.is_file() {
        return Err(Error::NotFound(
            "작품 폴더가 아님 (project.json이 없음)".into(),
        ));
    }
    let text = read_text(&path)?;
    let project: Project =
        serde_json::from_str(&text).map_err(|e| Error::format(&path, e.to_string()))?;
    if project.format > FORMAT_VERSION {
        return Err(Error::Invalid(
            "더 새로운 버전에서 만든 작품이라 열 수 없음. 앱을 업데이트해 주세요.".into(),
        ));
    }
    Ok(project)
}

pub fn save(root: &Path, project: &Project) -> Result<()> {
    let mut text = serde_json::to_string_pretty(project).expect("project serializes");
    text.push('\n');
    atomic_write(&root.join(PROJECT_FILE), text.as_bytes())
}

// ---------------------------------------------------------------------------
// Creating

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewProject {
    /// Folder the project folder is created in.
    pub parent: String,
    pub title: String,
    pub kind: ProjectKind,
    #[serde(default)]
    pub per_doc_goal: Option<u32>,
    #[serde(default = "yes")]
    pub count_spaces: bool,
    /// Create "1부" with a first chapter.
    #[serde(default = "yes")]
    pub first_chapter: bool,
}

/// Creates a new project folder inside `opts.parent` and returns its path.
pub fn create(opts: &NewProject) -> Result<PathBuf> {
    let title = opts.title.trim();
    if title.is_empty() {
        return Err(Error::Invalid("작품 제목을 적어 주세요".into()));
    }
    let parent = PathBuf::from(&opts.parent);
    fs::create_dir_all(&parent).map_err(|e| Error::io(&parent, e))?;
    let root = unique_dir(&parent, &safe_file_name(title, "새 작품"));
    for dir in [MANUSCRIPT_DIR, PLANNING_DIR] {
        let path = root.join(dir);
        fs::create_dir_all(&path).map_err(|e| Error::io(&path, e))?;
    }

    let mut part = new_part("1부");
    if opts.first_chapter {
        let id = write_new_doc(&root, Section::Manuscript, "")?;
        part.docs.push(id);
    }
    let mut planning = Vec::new();
    for name in ["시놉시스", "작품 소개"] {
        planning.push(write_new_doc(&root, Section::Planning, name)?);
    }

    let scene_break = match opts.kind {
        ProjectKind::Webnovel => "◆",
        ProjectKind::Print => "*",
    };
    let manuscript_format = format::default_for(opts.kind);
    let project = Project {
        app: APP_NAME.into(),
        format: FORMAT_VERSION,
        id: new_id(),
        title: title.into(),
        kind: opts.kind,
        pen_name: String::new(),
        created: now_iso(),
        goal: Goal {
            per_doc: opts.per_doc_goal.filter(|g| *g > 0),
            count_spaces: opts.count_spaces,
            daily: None,
        },
        scene_break: scene_break.into(),
        preset: manuscript_format.preset.clone(),
        manuscript_format: Some(manuscript_format),
        card_types: Some(cards::default_types()),
        parts: vec![part],
        planning,
        extra: Map::new(),
    };
    save(&root, &project)?;
    Ok(root)
}

// ---------------------------------------------------------------------------
// Moving

/// Where a project went, from `relocate`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Moved {
    pub root: String,
    /// The old folder could not be removed completely (a program was holding
    /// a file); what is left of it stays where it was.
    pub left_behind: bool,
}

/// Moves the project folder into `dest_parent`, for example into a folder
/// that OneDrive keeps in step with other devices. On the same disk this is
/// one rename. Otherwise the folder is copied, every file is checked against
/// the original, and only then is the old folder removed.
pub fn relocate(root: &Path, dest_parent: &Path) -> Result<Moved> {
    load(root)?;
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| Error::Invalid("작품 폴더가 올바르지 않음".into()))?;
    fs::create_dir_all(dest_parent).map_err(|e| Error::io(dest_parent, e))?;
    let (from, to) = (canonical(root)?, canonical(dest_parent)?);
    if to.starts_with(&from) {
        return Err(Error::Invalid("작품 폴더 안으로는 옮길 수 없음".into()));
    }
    if from.parent() == Some(to.as_path()) {
        return Err(Error::Invalid("이미 그 위치에 있음".into()));
    }
    let target = unique_dir(dest_parent, &name);

    if rename_dir(root, &target).is_ok() {
        return Ok(Moved {
            root: target.to_string_lossy().into_owned(),
            left_behind: false,
        });
    }

    let staging = dest_parent.join(format!(".{name}.{}.moving", new_id()));
    let copied = copy_tree(root, &staging).and_then(|()| same_tree(root, &staging));
    if let Err(e) = copied {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }
    if let Err(e) = rename_dir(&staging, &target) {
        let _ = fs::remove_dir_all(&staging);
        return Err(Error::io(&target, e));
    }
    Ok(Moved {
        root: target.to_string_lossy().into_owned(),
        left_behind: fs::remove_dir_all(root).is_err(),
    })
}

fn canonical(path: &Path) -> Result<PathBuf> {
    fs::canonicalize(path).map_err(|e| Error::io(path, e))
}

/// Renames a folder, retrying briefly while a sync program or virus scanner
/// holds a file in it.
fn rename_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    rename_retry(from, to, 4, Duration::from_millis(80))
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to).map_err(|e| Error::io(to, e))?;
    for entry in fs::read_dir(from).map_err(|e| Error::io(from, e))? {
        let entry = entry.map_err(|e| Error::io(from, e))?;
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        if entry
            .file_type()
            .map_err(|e| Error::io(&source, e))?
            .is_dir()
        {
            copy_tree(&source, &target)?;
        } else {
            fs::copy(&source, &target).map_err(|e| Error::io(&source, e))?;
        }
    }
    Ok(())
}

/// Checks that `copy` holds exactly the files of `original`, byte for byte.
fn same_tree(original: &Path, copy: &Path) -> Result<()> {
    for entry in fs::read_dir(original).map_err(|e| Error::io(original, e))? {
        let entry = entry.map_err(|e| Error::io(original, e))?;
        let (a, b) = (entry.path(), copy.join(entry.file_name()));
        if entry.file_type().map_err(|e| Error::io(&a, e))?.is_dir() {
            same_tree(&a, &b)?;
            continue;
        }
        let same = matches!(
            (fs::read(&a), fs::read(&b)),
            (Ok(x), Ok(y)) if x == y
        );
        if !same {
            return Err(Error::Invalid(
                "옮긴 파일이 원래 파일과 달라 옮기지 않음. 원래 폴더는 그대로 있습니다.".into(),
            ));
        }
    }
    Ok(())
}

fn unique_dir(parent: &Path, name: &str) -> PathBuf {
    let first = parent.join(name);
    if !first.exists() {
        return first;
    }
    (2..)
        .map(|n| parent.join(format!("{name} ({n})")))
        .find(|p| !p.exists())
        .expect("some name is free")
}

fn write_new_doc(root: &Path, section: Section, title: &str) -> Result<String> {
    let dir = root.join(section.dir());
    let id = loop {
        let id = new_id();
        if !dir.join(doc::file_name(&id)).exists() {
            break id;
        }
    };
    let doc = DocFile {
        meta: DocMeta::new(&id, title),
        body: Vec::new(),
    };
    doc::write_doc_file(&dir.join(doc::file_name(&id)), &doc)?;
    Ok(id)
}

// ---------------------------------------------------------------------------
// Overview

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
    pub id: String,
    pub title: String,
    pub kind: ProjectKind,
    pub pen_name: String,
    pub created: String,
    pub goal: Goal,
    pub scene_break: String,
    pub manuscript_format: ManuscriptFormat,
}

impl From<&Project> for ProjectInfo {
    fn from(p: &Project) -> Self {
        ProjectInfo {
            id: p.id.clone(),
            title: p.title.clone(),
            kind: p.kind,
            pen_name: p.pen_name.clone(),
            created: p.created.clone(),
            goal: p.goal.clone(),
            scene_break: p.scene_break.clone(),
            manuscript_format: p.manuscript_format(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocSummary {
    pub id: String,
    pub title: String,
    pub synopsis: String,
    pub status: String,
    pub target: Option<u32>,
    pub counts: Counts,
    /// Estimated pages in the manuscript format; none without paper.
    pub pages: Option<u32>,
    /// When the file was last written, for 개요 표.
    pub modified: Option<String>,
}

impl DocSummary {
    pub fn of(doc: &DocFile, metrics: Option<&PageMetrics>) -> Self {
        DocSummary {
            id: doc.meta.id.clone(),
            title: doc.meta.title.clone(),
            synopsis: doc.meta.synopsis.clone(),
            status: doc.meta.status.clone(),
            target: doc.meta.target,
            counts: count_blocks(&doc.body),
            pages: metrics.map(|m| m.chapter_pages(&doc.body, true)),
            modified: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PartView {
    pub id: String,
    pub title: String,
    pub docs: Vec<DocSummary>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub root: String,
    pub project: ProjectInfo,
    pub parts: Vec<PartView>,
    pub planning: Vec<DocSummary>,
    pub trash_count: usize,
    /// Manuscript totals (planning documents not included).
    pub total: Counts,
    /// Estimated pages of the whole manuscript; none without paper.
    pub total_pages: Option<u32>,
    pub card_types: Vec<CardType>,
    pub cards: Vec<CardSummary>,
    /// Copies left by sync programs, waiting for the writer to pick (copies.rs).
    pub copies: Vec<CopyInfo>,
}

/// Opens a project: sorts out what sync programs left (copies.rs), brings the
/// structure in line with the files on disk, clears expired trash and old
/// automatic records, and returns the overview.
pub fn open(root: &Path) -> Result<Overview> {
    if root.is_dir() {
        copies::restore_project_file(root)?;
    }
    let mut project = load(root)?;
    copies::reconcile(root)?;
    if copies::merge_project_copies(root)? {
        project = load(root)?;
    }
    if repair(root, &mut project)? {
        save(root, &project)?;
    }
    trash::purge(root, chrono::Duration::days(TRASH_DAYS))?;
    snapshot::prune(root, chrono::Duration::days(AUTO_RECORD_DAYS))?;
    overview(root)
}

/// Reads the documents the structure lists. One whose file is not there
/// (yet: it may still be on its way from another device) or cannot be read
/// right now is left out, as is a second mention of the same document.
fn listed_docs<'a>(
    root: &Path,
    section: Section,
    ids: impl IntoIterator<Item = &'a String>,
    seen: &mut HashSet<String>,
) -> Vec<(DocFile, Option<String>)> {
    ids.into_iter()
        .filter(|id| seen.insert((*id).clone()))
        .filter_map(|id| {
            let path = root.join(section.dir()).join(doc::file_name(id));
            let file = doc::read_doc(&path).ok()?;
            Some((file, modified_iso(&path)))
        })
        .collect()
}

/// Whether a document listed in the structure has its file.
fn present(root: &Path, section: Section, id: &str) -> bool {
    root.join(section.dir()).join(doc::file_name(id)).is_file()
}

pub fn overview(root: &Path) -> Result<Overview> {
    let project = load(root)?;
    let metrics = PageMetrics::of(&project.manuscript_format());
    let mut seen = HashSet::new();
    let mut total = Counts::default();
    let mut bodies: Vec<Vec<Block>> = Vec::new();
    let mut parts = Vec::with_capacity(project.parts.len());
    for part in &project.parts {
        let mut docs = Vec::with_capacity(part.docs.len());
        for (file, modified) in listed_docs(root, Section::Manuscript, &part.docs, &mut seen) {
            let mut summary = DocSummary::of(&file, metrics.as_ref());
            summary.modified = modified;
            total.add(&summary.counts);
            docs.push(summary);
            bodies.push(file.body);
        }
        parts.push(PartView {
            id: part.id.clone(),
            title: part.title.clone(),
            docs,
        });
    }
    let planning = listed_docs(root, Section::Planning, &project.planning, &mut seen)
        .into_iter()
        .map(|(file, modified)| {
            let mut summary = DocSummary::of(&file, None);
            summary.modified = modified;
            summary
        })
        .collect();
    Ok(Overview {
        root: root.to_string_lossy().into_owned(),
        project: ProjectInfo::from(&project),
        parts,
        planning,
        trash_count: trash::list(root)?.len(),
        total,
        total_pages: metrics.map(|m| m.pages(bodies.iter().map(Vec::as_slice), true)),
        card_types: project.card_types(),
        cards: cards::list(root)?,
        copies: copies::list(root)?,
    })
}

/// Estimated pages of the whole manuscript in `format`, for trying a format
/// before choosing it. None when the format has no paper.
pub fn estimate_pages(root: &Path, format: &ManuscriptFormat) -> Result<Option<u32>> {
    let Some(metrics) = PageMetrics::of(format) else {
        return Ok(None);
    };
    let project = load(root)?;
    let bodies: Vec<Vec<Block>> = listed_docs(
        root,
        Section::Manuscript,
        project.parts.iter().flat_map(|p| &p.docs),
        &mut HashSet::new(),
    )
    .into_iter()
    .map(|(file, _)| file.body)
    .collect();
    Ok(Some(metrics.pages(bodies.iter().map(Vec::as_slice), true)))
}

/// Document ids present as files in a section folder (copies left by sync
/// programs not included).
fn ids_on_disk(root: &Path, section: Section) -> Result<Vec<String>> {
    let dir = root.join(section.dir());
    fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    Ok(copies::scan(root, section)?.ids)
}

/// Makes the structure match the files: adds files the structure does not
/// list (for example ones that arrived through folder sync) and drops
/// entries listed twice. Entries whose file is missing stay: with folder sync
/// the structure can arrive before the chapter files, and the tree leaves
/// them out until they come. Returns whether anything changed.
fn repair(root: &Path, project: &mut Project) -> Result<bool> {
    let mut changed = false;
    if project.parts.is_empty() {
        project.parts.push(new_part("1부"));
        changed = true;
    }

    let on_disk: HashSet<String> = ids_on_disk(root, Section::Manuscript)?
        .into_iter()
        .collect();
    let mut seen = HashSet::new();
    for part in &mut project.parts {
        let before = part.docs.len();
        part.docs.retain(|id| seen.insert(id.clone()));
        changed |= part.docs.len() != before;
    }
    let mut found: Vec<String> = on_disk.difference(&seen).cloned().collect();
    if !found.is_empty() {
        sort_by_created(root, Section::Manuscript, &mut found);
        project
            .parts
            .last_mut()
            .expect("at least one part")
            .docs
            .extend(found);
        changed = true;
    }

    let on_disk: HashSet<String> = ids_on_disk(root, Section::Planning)?.into_iter().collect();
    let before = project.planning.len();
    project.planning.retain(|id| seen.insert(id.clone()));
    changed |= project.planning.len() != before;
    let mut found: Vec<String> = on_disk.difference(&seen).cloned().collect();
    if !found.is_empty() {
        sort_by_created(root, Section::Planning, &mut found);
        project.planning.extend(found);
        changed = true;
    }
    Ok(changed)
}

fn sort_by_created(root: &Path, section: Section, ids: &mut [String]) {
    ids.sort_by_cached_key(|id| {
        let path = root.join(section.dir()).join(doc::file_name(id));
        let created = doc::read_doc(&path)
            .map(|d| d.meta.created)
            .unwrap_or_default();
        (created, id.clone())
    });
}

// ---------------------------------------------------------------------------
// Editing the structure

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPatch {
    pub title: Option<String>,
    pub kind: Option<ProjectKind>,
    pub pen_name: Option<String>,
    pub goal: Option<Goal>,
    pub scene_break: Option<String>,
    pub manuscript_format: Option<ManuscriptFormat>,
}

pub fn update(root: &Path, patch: &ProjectPatch) -> Result<ProjectInfo> {
    let mut project = load(root)?;
    if let Some(title) = &patch.title {
        let title = title.trim();
        if title.is_empty() {
            return Err(Error::Invalid("작품 제목을 적어 주세요".into()));
        }
        project.title = title.into();
    }
    if let Some(kind) = patch.kind {
        project.kind = kind;
    }
    if let Some(pen_name) = &patch.pen_name {
        project.pen_name = pen_name.trim().into();
    }
    if let Some(goal) = &patch.goal {
        project.goal = goal.clone();
    }
    if let Some(symbol) = &patch.scene_break {
        let symbol = symbol.trim();
        if !symbol.is_empty() {
            project.scene_break = symbol.into();
        }
    }
    if let Some(format) = &patch.manuscript_format {
        format.validate()?;
        project.preset = format.preset.clone();
        project.manuscript_format = Some(format.clone());
    }
    save(root, &project)?;
    Ok(ProjectInfo::from(&project))
}

pub fn add_part(root: &Path, title: &str) -> Result<String> {
    let mut project = load(root)?;
    let title = match title.trim() {
        "" => format!("{}부", project.parts.len() + 1),
        t => t.to_string(),
    };
    let part = new_part(&title);
    let id = part.id.clone();
    project.parts.push(part);
    save(root, &project)?;
    Ok(id)
}

pub fn rename_part(root: &Path, part_id: &str, title: &str) -> Result<()> {
    let mut project = load(root)?;
    let title = title.trim();
    if title.is_empty() {
        return Err(Error::Invalid("부 이름을 적어 주세요".into()));
    }
    project.part_mut(part_id)?.title = title.into();
    save(root, &project)
}

/// Removes an empty part. A part that still has chapters cannot be removed.
pub fn remove_part(root: &Path, part_id: &str) -> Result<()> {
    let mut project = load(root)?;
    let part = project.part_mut(part_id)?;
    // Entries whose file is gone do not count (see `repair`).
    part.docs
        .retain(|id| present(root, Section::Manuscript, id));
    if !part.docs.is_empty() {
        return Err(Error::Invalid(
            "회차가 남아 있는 부는 지울 수 없음. 회차를 먼저 옮기거나 휴지통으로 보내 주세요."
                .into(),
        ));
    }
    if project.parts.len() == 1 {
        return Err(Error::Invalid("부가 하나뿐이라 지울 수 없음".into()));
    }
    project.parts.retain(|p| p.id != part_id);
    save(root, &project)
}

/// Where a new document goes.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewDoc {
    pub section: Option<Section>,
    /// Manuscript only. Defaults to the last part.
    pub part_id: Option<String>,
    /// Insert right after this document; at the end when missing.
    pub after: Option<String>,
    #[serde(default)]
    pub title: String,
}

pub fn add_doc(root: &Path, spec: &NewDoc) -> Result<String> {
    let mut project = load(root)?;
    let section = spec.section.unwrap_or(Section::Manuscript);
    if matches!(section, Section::Cards | Section::Notes) {
        return Err(Error::Invalid("설정 카드와 메모는 따로 만듭니다".into()));
    }
    let id = write_new_doc(root, section, spec.title.trim())?;
    match section {
        Section::Cards | Section::Notes => unreachable!("checked above"),
        Section::Planning => {
            let index = spec
                .after
                .as_ref()
                .and_then(|a| project.planning.iter().position(|d| d == a))
                .map_or(project.planning.len(), |i| i + 1);
            project.planning.insert(index, id.clone());
        }
        Section::Manuscript => place_docs(
            &mut project,
            std::slice::from_ref(&id),
            spec.part_id.as_deref(),
            spec.after.as_deref(),
        ),
    }
    save(root, &project)?;
    Ok(id)
}

/// Puts chapters into the manuscript: right after `after`, or else at the end
/// of `part_id`, or else at the end of the last part.
pub(crate) fn place_docs(
    project: &mut Project,
    ids: &[String],
    part_id: Option<&str>,
    after: Option<&str>,
) {
    if project.parts.is_empty() {
        project.parts.push(new_part("1부"));
    }
    let after_pos = after.and_then(|a| {
        project
            .parts
            .iter()
            .enumerate()
            .find_map(|(pi, p)| p.docs.iter().position(|d| d == a).map(|di| (pi, di + 1)))
    });
    let (part_index, index) = match after_pos {
        Some(pos) => pos,
        None => {
            let pi = part_id
                .and_then(|id| project.parts.iter().position(|p| p.id == id))
                .unwrap_or(project.parts.len() - 1);
            (pi, project.parts[pi].docs.len())
        }
    };
    project.parts[part_index]
        .docs
        .splice(index..index, ids.iter().cloned());
}

/// Moves a chapter to `index` inside `part_id` (index counted after removal),
/// or a planning document to `index` when `part_id` is `None`.
pub fn move_doc(root: &Path, doc_id: &str, part_id: Option<&str>, index: usize) -> Result<()> {
    let mut project = load(root)?;
    let Some((from_part, _)) = project.detach(doc_id) else {
        return Err(Error::NotFound("문서를 찾을 수 없음".into()));
    };
    match (from_part, part_id) {
        (Some(_), Some(part_id)) => {
            let part = project.part_mut(part_id)?;
            let index = raw_index(root, Section::Manuscript, &part.docs, index);
            part.docs.insert(index, doc_id.into());
        }
        (None, None) => {
            let index = raw_index(root, Section::Planning, &project.planning, index);
            project.planning.insert(index, doc_id.into());
        }
        _ => {
            return Err(Error::Invalid(
                "원고와 기획 문서 사이에서는 옮길 수 없음".into(),
            ));
        }
    }
    save(root, &project)
}

/// Turns a position among the documents shown in the tree into a position in
/// `ids`, which may also list documents whose file has not arrived yet.
fn raw_index(root: &Path, section: Section, ids: &[String], shown_index: usize) -> usize {
    let mut shown = 0;
    for (i, id) in ids.iter().enumerate() {
        if !present(root, section, id) {
            continue;
        }
        if shown == shown_index {
            return i;
        }
        shown += 1;
    }
    ids.len()
}

/// Every document id in `project.json` order, manuscript first.
pub fn doc_ids(project: &Project) -> Vec<String> {
    project
        .parts
        .iter()
        .flat_map(|p| p.docs.iter().cloned())
        .chain(project.planning.iter().cloned())
        .collect()
}
