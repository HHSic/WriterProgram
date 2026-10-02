//! Creating a project folder.

use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Map;

use super::{
    APP_NAME, FORMAT_VERSION, Goal, MANUSCRIPT_DIR, PLANNING_DIR, Project, ProjectKind, new_part,
    save, unique_dir, yes,
};
use crate::cards;
use crate::doc::{self, DocFile, DocMeta, Section};
use crate::format;
use crate::store::{new_id, now_iso, safe_file_name};
use crate::{Error, Result};

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
        keep_daily: false,
        extra: Map::new(),
    };
    save(&root, &project)?;
    Ok(root)
}

/// Writes an empty document with a new id into its section folder.
pub(super) fn write_new_doc(root: &Path, section: Section, title: &str) -> Result<String> {
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
