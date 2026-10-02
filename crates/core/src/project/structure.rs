//! Editing the structure: project settings, parts, and where documents go.

use std::path::Path;

use serde::Deserialize;

use super::create::write_new_doc;
use super::overview::{ProjectInfo, present};
use super::{Goal, Project, ProjectKind, load, new_part, save};
use crate::doc::Section;
use crate::format::ManuscriptFormat;
use crate::{Error, Result};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPatch {
    pub title: Option<String>,
    pub kind: Option<ProjectKind>,
    pub pen_name: Option<String>,
    pub goal: Option<Goal>,
    pub scene_break: Option<String>,
    pub manuscript_format: Option<ManuscriptFormat>,
    pub keep_daily: Option<bool>,
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
    if let Some(keep) = patch.keep_daily {
        project.keep_daily = keep;
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
