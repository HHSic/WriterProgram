//! Opening a project: what the app shows of it, and bringing the structure
//! in line with the files on disk.

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde::Serialize;

use super::{
    AUTO_RECORD_DAYS, Goal, Platform, Project, ProjectKind, TRASH_DAYS, load, new_part, save,
};
use crate::cards::{self, CardSummary, CardType};
use crate::copies::{self, CopyInfo};
use crate::count::{Counts, count_blocks};
use crate::doc::{self, DocFile, Section};
use crate::format::ManuscriptFormat;
use crate::layout::PageMetrics;
use crate::markup::Block;
use crate::store::{Cleanup, modified_iso};
use crate::{Error, Result, snapshot, trash};

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
    /// 창작 과정 보관 (`Project::keep_daily`).
    pub keep_daily: bool,
    /// 연재 플랫폼 (`Project::platform`).
    pub platform: Option<Platform>,
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
            keep_daily: p.keep_daily,
            platform: p.platform.clone(),
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
    /// Copies left by sync programs, waiting for the writer to pick (copies/).
    pub copies: Vec<CopyInfo>,
}

/// Opens a project: sorts out what sync programs left (copies/), brings the
/// structure in line with the files on disk, clears expired trash and old
/// automatic records, and returns the overview.
///
/// The steps that guard the manuscript stop the opening when they fail, with
/// a reason in the writer's words. Clearing comes last and never stops it:
/// what cannot be removed now is only logged and tried again next time.
pub fn open(root: &Path) -> Result<Overview> {
    if root.is_dir() {
        guard(
            "다른 기기의 작품 구조 파일을 되살리지 못함",
            copies::restore_project_file(root),
        )?;
    }
    let mut project = load(root)?;
    guard(
        "동기화 프로그램이 남긴 사본을 정리하지 못함",
        copies::reconcile(root),
    )?;
    if guard(
        "다른 기기의 작품 구조를 합치지 못함",
        copies::merge_project_copies(root),
    )? {
        project = load(root)?;
    }
    let what = "작품 구조를 원고 파일과 맞추지 못함";
    if guard(what, repair(root, &mut project))? {
        guard(what, save(root, &project))?;
    }
    clean_up(root, project.keep_daily);
    overview(root)
}

/// Puts the step that failed in front of the reason, and the file name after
/// it: "사본을 정리하지 못함 · 이 위치에 쓸 권한이 없음 (k7q2m9x4t1ab.md)".
fn guard<T>(what: &str, result: Result<T>) -> Result<T> {
    result.map_err(|e| {
        let file = match &e {
            Error::Io { path, .. } => path
                .file_name()
                .map(|n| format!(" ({})", n.to_string_lossy()))
                .unwrap_or_default(),
            _ => String::new(),
        };
        Error::Invalid(format!("{what} · {}{file}", e.user_message()))
    })
}

/// Clears expired trash and old automatic records. Nothing for the writer to
/// do when a file is held by another program, so a failure is only logged.
fn clean_up(root: &Path, keep_daily: bool) -> Cleanup {
    let mut done = trash::purge(root, chrono::Duration::days(TRASH_DAYS));
    done.add(snapshot::prune(
        root,
        chrono::Duration::days(AUTO_RECORD_DAYS),
        keep_daily,
    ));
    if !done.failed.is_empty() {
        let names: Vec<String> = done
            .failed
            .iter()
            .map(|p| p.display().to_string())
            .collect();
        eprintln!(
            "정리하지 못한 항목 {}개(다음에 작품을 열 때 다시 시도): {}",
            done.failed.len(),
            names.join(", ")
        );
    }
    done
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
pub(super) fn present(root: &Path, section: Section, id: &str) -> bool {
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
