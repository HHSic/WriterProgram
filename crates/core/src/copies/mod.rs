//! Copies (사본) that sync programs leave when the same file was changed on two
//! devices before they caught up with each other. Each program names them its
//! own way, starting with the original name:
//!
//! | program | copy of `k7q2m9x4t1ab.md` |
//! |---|---|
//! | OneDrive | `k7q2m9x4t1ab-DESKTOP-1AB2C3D.md` (computer name) |
//! | Dropbox | `k7q2m9x4t1ab (홍길동's conflicted copy 2026-09-28).md` |
//! | iCloud Drive | `k7q2m9x4t1ab 2.md` |
//! | Google Drive | `k7q2m9x4t1ab (1).md` |
//! | Syncthing | `k7q2m9x4t1ab.sync-conflict-20260928-101500-ABCDEF7.md` |
//!
//! The front matter inside a copy still carries the original id, which is how
//! copies are told apart from other files. They are kept out of the tree, the
//! setting cards and the notes; the app shows them next to the original so the
//! writer can compare and pick (`resolve`). A copy of `project.json` is merged
//! into it (`merge_project_copies`).
//!
//! Two more cases are sorted out when a project opens (`reconcile`): a copy
//! whose original is gone takes the original's place, and a manuscript or
//! planning file whose name is not an id (dropped in by hand, say `메모.md`)
//! is taken in under an id worked out from its name, so every device picks
//! the same one.

mod merge;
mod scan;
#[cfg(test)]
mod tests;

pub(crate) use merge::project_copies;
pub use merge::{merge_project, merge_project_copies, reconcile, restore_project_file};
pub(crate) use scan::scan;
pub use scan::{device_of, is_id};

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::cards::{self, parse_card, write_card};
use crate::count::count_blocks;
use crate::doc::{self, DocFile, Section};
use crate::notes::{self, parse_note};
use crate::project::{self, Project};
use crate::snapshot;
use crate::store::{atomic_write, modified_iso, new_id, read_text, stamp};
use crate::trash;
use crate::{Error, Result};

/// Sections whose files can have copies.
pub const SECTIONS: [Section; 4] = [
    Section::Manuscript,
    Section::Planning,
    Section::Cards,
    Section::Notes,
];

/// A copy left by a sync program, as shown to the writer.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyInfo {
    /// File name in the section folder, e.g. `k7q2m9x4t1ab-DESKTOP-1AB2C3D.md`.
    pub file: String,
    pub section: Section,
    /// Id of the document, card or note it is a copy of.
    pub of: String,
    /// Title (documents), name (cards) or first line (notes) in the copy.
    pub title: String,
    /// When the copy was last written.
    pub modified: Option<String>,
    /// Length of the copy's text, spaces included.
    pub chars: u32,
    /// The device the copy came from, when the sync program named it.
    pub device: Option<String>,
}

fn file_path(root: &Path, section: Section, file: &str) -> Result<PathBuf> {
    let plain = !file.is_empty()
        && file.ends_with(".md")
        && !file.starts_with('.')
        && !file.contains(['/', '\\'])
        && !file.contains("..");
    if !plain {
        return Err(Error::Invalid("올바르지 않은 사본 이름".into()));
    }
    Ok(root.join(section.dir()).join(file))
}

/// The original a copy belongs to; an error when `file` is not a copy.
fn original_of(root: &Path, section: Section, file: &str) -> Result<String> {
    file_path(root, section, file)?;
    scan(root, section)?
        .copies
        .into_iter()
        .find(|(f, _)| f == file)
        .map(|(_, of)| of)
        .ok_or_else(|| Error::NotFound("사본을 찾을 수 없음".into()))
}

fn describe(root: &Path, section: Section, file: &str, of: &str) -> Result<CopyInfo> {
    let path = file_path(root, section, file)?;
    let text = read_text(&path)?;
    let stem = file.strip_suffix(".md").unwrap_or(file);
    let (title, chars) = match section {
        Section::Manuscript | Section::Planning => {
            let d = doc::parse_doc(&text, of);
            (d.meta.title, count_blocks(&d.body).with_spaces)
        }
        Section::Cards => {
            let c = parse_card(&text, of);
            (c.name, c.description.chars().count() as u32)
        }
        Section::Notes => {
            let n = parse_note(&text, of);
            let first = n
                .text
                .lines()
                .find(|l| !l.trim().is_empty())
                .unwrap_or(&n.quote)
                .chars()
                .take(40)
                .collect();
            (first, n.text.chars().count() as u32)
        }
    };
    Ok(CopyInfo {
        file: file.to_string(),
        section,
        of: of.to_string(),
        title,
        modified: modified_iso(&path),
        chars,
        device: device_of(stem, of),
    })
}

/// Every copy in the project, by section and file name.
pub fn list(root: &Path) -> Result<Vec<CopyInfo>> {
    let mut out = Vec::new();
    for section in SECTIONS {
        for (file, of) in scan(root, section)?.copies {
            // A copy still being written by the sync program is skipped for now.
            if let Ok(info) = describe(root, section, &file, &of) {
                out.push(info);
            }
        }
    }
    Ok(out)
}

/// A copy of a manuscript or planning document, to compare with the original.
pub fn load(root: &Path, section: Section, file: &str) -> Result<DocFile> {
    if !matches!(section, Section::Manuscript | Section::Planning) {
        return Err(Error::Invalid(
            "원고와 기획 문서의 사본만 비교할 수 있음".into(),
        ));
    }
    let of = original_of(root, section, file)?;
    let text = read_text(&file_path(root, section, file)?)?;
    let mut copy = doc::parse_doc(&text, &of);
    copy.meta.id = of;
    Ok(copy)
}

/// What to do with a copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Resolve {
    /// The copy replaces the original. The original's text is kept: as a
    /// record for documents, in the trash for cards and notes.
    Take,
    /// The copy goes to the trash.
    Discard,
    /// The copy becomes a document or card of its own, next to the original.
    KeepBoth,
}

pub fn resolve(root: &Path, section: Section, file: &str, action: Resolve) -> Result<()> {
    let of = original_of(root, section, file)?;
    let copy_path = file_path(root, section, file)?;
    let original_path = root.join(section.dir()).join(doc::file_name(&of));
    match (action, section) {
        (Resolve::Discard, _) => {
            let info = describe(root, section, file, &of)?;
            trash::trash_copy(root, section, file, &of, &info.title, info.chars)?;
        }
        (Resolve::Take, Section::Manuscript | Section::Planning) => {
            let original = doc::read_doc(&original_path)?;
            let copy = doc::parse_doc(&read_text(&copy_path)?, &of);
            snapshot::keep_unless_same(root, &original, "before-copy")?;
            let mut meta = copy.meta;
            meta.id = of.clone();
            meta.created = original.meta.created;
            doc::write_doc_file(
                &original_path,
                &DocFile {
                    meta,
                    body: copy.body,
                },
            )?;
            // Its text is the document's now.
            fs::remove_file(&copy_path).map_err(|e| Error::io(&copy_path, e))?;
        }
        (Resolve::Take, Section::Cards | Section::Notes) => {
            // The two swap places: the current one waits in the trash as a copy.
            let current = describe_file(root, section, &of)?;
            let aside = format!("{of} (바꾸기 전 {}).md", stamp(Utc::now()));
            trash::trash_copy_as(
                root,
                section,
                &doc::file_name(&of),
                &aside,
                &of,
                &current.0,
                current.1,
            )?;
            rename(&copy_path, &original_path)?;
        }
        (Resolve::KeepBoth, Section::Manuscript | Section::Planning) => {
            let text = read_text(&copy_path)?;
            let mut copy = doc::parse_doc(&text, &of);
            let id = unused_id(root, section);
            copy.meta.id = id.clone();
            copy.meta.title = format!("{} (사본)", copy.meta.title.trim())
                .trim()
                .to_string();
            doc::write_doc_file(&root.join(section.dir()).join(doc::file_name(&id)), &copy)?;
            let mut p = project::load(root)?;
            place_after(&mut p, &of, &id, section);
            project::save(root, &p)?;
            fs::remove_file(&copy_path).map_err(|e| Error::io(&copy_path, e))?;
        }
        (Resolve::KeepBoth, Section::Cards) => {
            let mut card = parse_card(&read_text(&copy_path)?, &of);
            card.id = unused_id(root, section);
            card.name = format!("{} (사본)", card.name.trim());
            atomic_write(
                &root.join(section.dir()).join(doc::file_name(&card.id)),
                write_card(&card).as_bytes(),
            )?;
            fs::remove_file(&copy_path).map_err(|e| Error::io(&copy_path, e))?;
        }
        (Resolve::KeepBoth, Section::Notes) => {
            return Err(Error::Invalid(
                "메모 사본은 따로 둘 수 없음. 둘 중 하나를 골라 주세요.".into(),
            ));
        }
    }
    Ok(())
}

/// Title (or name) and length of a card or note file.
fn describe_file(root: &Path, section: Section, id: &str) -> Result<(String, u32)> {
    match section {
        Section::Cards => {
            let c = cards::load(root, id)?;
            Ok((c.name, c.description.chars().count() as u32))
        }
        Section::Notes => {
            let n = notes::load(root, id)?;
            let first: String = n
                .text
                .lines()
                .next()
                .unwrap_or(&n.quote)
                .chars()
                .take(40)
                .collect();
            Ok((first, n.text.chars().count() as u32))
        }
        Section::Manuscript | Section::Planning => {
            let d = doc::load(root, id)?;
            Ok((d.meta.title.clone(), count_blocks(&d.body).with_spaces))
        }
    }
}

fn unused_id(root: &Path, section: Section) -> String {
    let dir = root.join(section.dir());
    loop {
        let id = new_id();
        let free = match section {
            // Manuscript and planning ids must be unique across both folders.
            Section::Manuscript | Section::Planning => doc::locate(root, &id).is_err(),
            Section::Cards | Section::Notes => !dir.join(doc::file_name(&id)).exists(),
        };
        if free {
            return id;
        }
    }
}

/// Puts `id` right after `after` in the structure (at the end when `after`
/// is not there).
fn place_after(p: &mut Project, after: &str, id: &str, section: Section) {
    if section == Section::Planning {
        let at = p
            .planning
            .iter()
            .position(|d| d == after)
            .map_or(p.planning.len(), |i| i + 1);
        p.planning.insert(at, id.to_string());
        return;
    }
    for part in &mut p.parts {
        if let Some(i) = part.docs.iter().position(|d| d == after) {
            part.docs.insert(i + 1, id.to_string());
            return;
        }
    }
    p.attach_manuscript(id, None, usize::MAX);
}

fn rename(from: &Path, to: &Path) -> Result<()> {
    fs::rename(from, to).map_err(|e| Error::io(from, e))
}
