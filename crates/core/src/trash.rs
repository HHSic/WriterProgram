//! Trash (휴지통): deleted documents are moved to `.trash/<item id>/` together
//! with an `item.json` that remembers where they were, and kept 30 days.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::count::count_blocks;
use crate::doc::{self, Section};
use crate::store::{atomic_write, parse_iso, read_text, stamp, to_iso};
use crate::{Error, Result, cards, notes, project, snapshot};

pub const TRASH_DIR: &str = ".trash";
const ITEM_FILE: &str = "item.json";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashItem {
    pub id: String,
    pub doc_id: String,
    pub section: Section,
    pub title: String,
    pub deleted_at: String,
    /// Part the chapter was in, and its position there.
    #[serde(default)]
    pub part_id: Option<String>,
    #[serde(default)]
    pub index: usize,
    #[serde(default)]
    pub chars: u32,
}

fn item_dir(root: &Path, id: &str) -> Result<PathBuf> {
    let ok = !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !ok {
        return Err(Error::Invalid("올바르지 않은 휴지통 항목".into()));
    }
    Ok(root.join(TRASH_DIR).join(id))
}

/// Moves a document to the trash and takes it out of the structure.
pub fn trash_doc(root: &Path, doc_id: &str) -> Result<TrashItem> {
    let mut project = project::load(root)?;
    let (section, path) = doc::locate(root, doc_id)?;
    let file = doc::read_doc(&path)?;
    let (part_id, index) = project.detach(doc_id).unwrap_or((None, 0));

    let now = Utc::now();
    let id = format!("{}-{doc_id}", stamp(now));
    let dir = item_dir(root, &id)?;
    fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    let item = TrashItem {
        id,
        doc_id: doc_id.into(),
        section,
        title: file.meta.title.clone(),
        deleted_at: to_iso(now),
        part_id,
        index,
        chars: count_blocks(&file.body).with_spaces,
    };
    let target = dir.join(doc::file_name(doc_id));
    fs::rename(&path, &target).map_err(|e| Error::io(&path, e))?;
    write_item(&dir, &item)?;
    project::save(root, &project)?;
    Ok(item)
}

fn write_item(dir: &Path, item: &TrashItem) -> Result<()> {
    let text = serde_json::to_string_pretty(item).expect("item serializes");
    atomic_write(&dir.join(ITEM_FILE), text.as_bytes())
}

fn read_item(root: &Path, id: &str) -> Result<TrashItem> {
    let dir = item_dir(root, id)?;
    let path = dir.join(ITEM_FILE);
    if let Ok(text) = read_text(&path)
        && let Ok(item) = serde_json::from_str::<TrashItem>(&text)
    {
        return Ok(item);
    }
    // No item.json (interrupted move): rebuild what we can from the file.
    let entries = fs::read_dir(&dir).map_err(|e| Error::io(&dir, e))?;
    let md = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| p.extension().is_some_and(|x| x == "md"))
        .ok_or_else(|| Error::NotFound("휴지통 항목이 비어 있음".into()))?;
    let file = doc::read_doc(&md)?;
    let deleted_at = crate::store::parse_stamp(id)
        .map(to_iso)
        .unwrap_or_default();
    Ok(TrashItem {
        id: id.into(),
        doc_id: file.meta.id.clone(),
        section: Section::Manuscript,
        title: file.meta.title.clone(),
        deleted_at,
        part_id: None,
        index: usize::MAX,
        chars: count_blocks(&file.body).with_spaces,
    })
}

/// Items in the trash, most recently deleted first.
pub fn list(root: &Path) -> Result<Vec<TrashItem>> {
    let base = root.join(TRASH_DIR);
    let entries = match fs::read_dir(&base) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(&base, e)),
    };
    let mut items = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        if !entry.path().is_dir() {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        if let Ok(item) = read_item(root, &id) {
            items.push(item);
        }
    }
    items.sort_by(|a, b| b.deleted_at.cmp(&a.deleted_at));
    Ok(items)
}

/// Moves a setting card to the trash.
pub fn trash_card(root: &Path, card_id: &str) -> Result<TrashItem> {
    let card = cards::load(root, card_id)?;
    let path = root
        .join(Section::Cards.dir())
        .join(doc::file_name(card_id));
    let now = Utc::now();
    let id = format!("{}-{card_id}", stamp(now));
    let dir = item_dir(root, &id)?;
    fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    let item = TrashItem {
        id,
        doc_id: card_id.into(),
        section: Section::Cards,
        title: card.name.clone(),
        deleted_at: to_iso(now),
        part_id: None,
        index: 0,
        chars: card.description.chars().count() as u32,
    };
    fs::rename(&path, dir.join(doc::file_name(card_id))).map_err(|e| Error::io(&path, e))?;
    write_item(&dir, &item)?;
    Ok(item)
}

/// Moves a note to the trash. A note on a stretch of text loses its mark in
/// the document, so a note brought back later shows the text it was on.
pub fn trash_note(root: &Path, note_id: &str) -> Result<TrashItem> {
    let note = notes::load(root, note_id)?;
    notes::unmark_file(root, &note)?;
    let path = notes::path(root, note_id)?;
    let now = Utc::now();
    let id = format!("{}-{note_id}", stamp(now));
    let dir = item_dir(root, &id)?;
    fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    let first_line = note
        .text
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(&note.quote);
    let item = TrashItem {
        id,
        doc_id: note_id.into(),
        section: Section::Notes,
        title: first_line.chars().take(40).collect(),
        deleted_at: to_iso(now),
        part_id: None,
        index: 0,
        chars: note.text.chars().count() as u32,
    };
    fs::rename(&path, dir.join(doc::file_name(note_id))).map_err(|e| Error::io(&path, e))?;
    write_item(&dir, &item)?;
    Ok(item)
}

/// Puts a document back where it was (or at the end when that place is gone).
pub fn restore(root: &Path, id: &str) -> Result<TrashItem> {
    let item = read_item(root, id)?;
    let dir = item_dir(root, id)?;
    let target = root
        .join(item.section.dir())
        .join(doc::file_name(&item.doc_id));
    if target.exists() {
        return Err(Error::Invalid(
            "같은 문서가 이미 원고에 있어 되살릴 수 없음".into(),
        ));
    }
    let source = dir.join(doc::file_name(&item.doc_id));
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    }
    fs::rename(&source, &target).map_err(|e| Error::io(&source, e))?;

    if matches!(item.section, Section::Cards | Section::Notes) {
        fs::remove_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
        return Ok(item);
    }
    let mut project = project::load(root)?;
    match item.section {
        Section::Cards | Section::Notes => {}
        Section::Manuscript => {
            project.attach_manuscript(&item.doc_id, item.part_id.as_deref(), item.index)
        }
        Section::Planning => {
            let index = item.index.min(project.planning.len());
            project.planning.insert(index, item.doc_id.clone());
        }
    }
    project::save(root, &project)?;
    fs::remove_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    Ok(item)
}

/// Deletes an item for good, including the document's records.
pub fn delete(root: &Path, id: &str) -> Result<()> {
    let item = read_item(root, id)?;
    let dir = item_dir(root, id)?;
    fs::remove_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    snapshot::remove_all(root, &item.doc_id)
}

/// Deletes items older than `keep`.
pub fn purge(root: &Path, keep: Duration) -> Result<()> {
    let cutoff = Utc::now() - keep;
    for item in list(root)? {
        if parse_iso(&item.deleted_at).is_some_and(|t| t < cutoff) {
            delete(root, &item.id)?;
        }
    }
    Ok(())
}
