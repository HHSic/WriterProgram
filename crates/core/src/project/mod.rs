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

mod create;
mod overview;
mod relocate;
mod size;
mod structure;

pub use create::{NewProject, create};
pub use overview::{DocSummary, Overview, PartView, ProjectInfo, estimate_pages, open, overview};
pub use relocate::{Moved, relocate};
pub use size::{JOURNAL_DIR, Sizes, disk_free, sizes, tidy_records};
pub(crate) use structure::place_docs;
pub use structure::{
    NewDoc, ProjectPatch, add_doc, add_part, doc_ids, move_doc, remove_part, rename_part, update,
};

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::cards::{self, CardType};
use crate::count::CountRule;
use crate::format::{self, ManuscriptFormat};
use crate::store::{atomic_write, new_id, read_text};
use crate::{Error, Result};

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

/// 연재 플랫폼: where a web novel is serialized, and how that platform counts
/// characters. The rule is kept with the project so the writer can change it
/// when a platform changes its counter (docs/platforms.md).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    /// `munpia`, `novelpia`, `kakaopage`, `naver`, `ridi` or `custom`
    /// (app/src/lib/platforms.ts).
    pub id: String,
    #[serde(default)]
    pub rule: CountRule,
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
    /// Keeps each chapter's last record of every day when old automatic
    /// records are cleared (창작 과정 보관, docs/creation-proof.md §4.3).
    /// Off unless the writer turns it on.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub keep_daily: bool,
    /// 연재 플랫폼 whose counting chapter counts and goals follow. Web novels only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform: Option<Platform>,
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

/// `name` inside `parent`, or `name (2)`, `name (3)` … when it is taken.
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
