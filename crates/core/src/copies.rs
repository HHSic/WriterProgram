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

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::cards::{self, parse_card, write_card};
use crate::count::count_blocks;
use crate::doc::{self, DocFile, Section, decode_value};
use crate::notes::{self, parse_note};
use crate::project::{self, PROJECT_FILE, Part, Project};
use crate::snapshot::{self, SNAPSHOT_DIR};
use crate::store::{atomic_write, new_id, read_text, stable_id, stamp, to_iso};
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

/// Whether `s` can be a file id: letters, digits, '-' and '_' only.
pub fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// One `.md` file in a section folder.
#[derive(Debug, Clone)]
struct Found {
    file: String,
    stem: String,
    /// The id in its front matter, if it has one.
    front_id: Option<String>,
    modified: Option<std::time::SystemTime>,
}

/// What the files of one section folder are.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Scan {
    /// Ids of the files named after their own id.
    pub ids: Vec<String>,
    /// Copies: (file name, id of the original).
    pub copies: Vec<(String, String)>,
    /// Copies whose original is gone: (file name, id they take over).
    pub orphans: Vec<(String, String)>,
    /// Files that are not ids and not copies.
    pub loose: Vec<String>,
}

/// Reads the id line of a file's front matter (it is always near the top).
fn front_id(path: &Path) -> Option<String> {
    let mut head = Vec::with_capacity(1024);
    File::open(path)
        .ok()?
        .take(1024)
        .read_to_end(&mut head)
        .ok()?;
    let text = String::from_utf8_lossy(&head).replace("\r\n", "\n");
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let rest = text.strip_prefix("---\n")?;
    for line in rest.lines() {
        if line == "---" {
            break;
        }
        if let Some(raw) = line.strip_prefix("id:") {
            let id = decode_value(raw.trim());
            return (!id.is_empty()).then_some(id);
        }
    }
    None
}

fn read_section(root: &Path, section: Section) -> Result<Vec<Found>> {
    let dir = root.join(section.dir());
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(&dir, e)),
    };
    let mut found = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let file = entry.file_name().to_string_lossy().into_owned();
        if file.starts_with('.') || !entry.path().is_file() {
            continue;
        }
        let Some(stem) = file.strip_suffix(".md") else {
            continue;
        };
        found.push(Found {
            stem: stem.to_string(),
            front_id: front_id(&entry.path()),
            modified: entry.metadata().ok().and_then(|m| m.modified().ok()),
            file,
        });
    }
    // Same order on every run and every device.
    found.sort_by(|a, b| a.file.cmp(&b.file));
    Ok(found)
}

/// Sorts the files of a folder into originals, copies and the rest.
fn classify(found: &[Found]) -> Scan {
    let mut scan = Scan::default();
    let mut ids: HashSet<&str> = HashSet::new();
    for f in found {
        if is_id(&f.stem) && f.front_id.as_deref().is_none_or(|id| id == f.stem) {
            ids.insert(&f.stem);
            scan.ids.push(f.stem.clone());
        }
    }
    // Copies whose original is gone, newest first: the newest takes its place.
    let mut orphans: HashMap<String, Vec<&Found>> = HashMap::new();
    for f in found {
        if ids.contains(f.stem.as_str()) {
            continue;
        }
        let of = f
            .front_id
            .clone()
            .filter(|id| is_id(id))
            .or_else(|| prefix_id(&f.stem, &ids));
        match of {
            Some(of) if ids.contains(of.as_str()) => scan.copies.push((f.file.clone(), of)),
            Some(of) => orphans.entry(of).or_default().push(f),
            None => scan.loose.push(f.file.clone()),
        }
    }
    let mut orphans: Vec<_> = orphans.into_iter().collect();
    orphans.sort_by(|a, b| a.0.cmp(&b.0));
    for (of, mut files) in orphans {
        files.sort_by(|a, b| b.modified.cmp(&a.modified).then(a.file.cmp(&b.file)));
        scan.orphans.push((files[0].file.clone(), of.clone()));
        for f in &files[1..] {
            scan.copies.push((f.file.clone(), of.clone()));
        }
    }
    scan
}

/// The id a copy's name starts with, for copies without front matter.
fn prefix_id(stem: &str, ids: &HashSet<&str>) -> Option<String> {
    ids.iter()
        .filter(|id| {
            stem.len() > id.len()
                && stem.starts_with(**id)
                && !stem[id.len()..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        })
        .max_by_key(|id| id.len())
        .map(|id| id.to_string())
}

pub(crate) fn scan(root: &Path, section: Section) -> Result<Scan> {
    Ok(classify(&read_section(root, section)?))
}

/// The device named in a copy's file name, when the sync program put it there.
pub fn device_of(stem: &str, of: &str) -> Option<String> {
    let rest = stem.strip_prefix(of)?;
    // Dropbox: " (홍길동's conflicted copy 2026-09-28)".
    if let Some(inner) = rest
        .trim()
        .strip_prefix('(')
        .and_then(|r| r.strip_suffix(')'))
        && let Some(i) = inner.find("conflicted copy")
    {
        let who = inner[..i].trim().trim_end_matches("'s").trim();
        return (!who.is_empty()).then(|| who.to_string());
    }
    // OneDrive: "-DESKTOP-1AB2C3D", "-DESKTOP-1AB2C3D-2".
    let name = rest.strip_prefix('-')?;
    let name = match name.rsplit_once('-') {
        Some((head, n)) if !head.is_empty() && n.chars().all(|c| c.is_ascii_digit()) => head,
        _ => name,
    };
    let looks_like_computer = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    looks_like_computer.then(|| name.to_string())
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

fn modified_iso(path: &Path) -> Option<String> {
    let time = fs::metadata(path).ok()?.modified().ok()?;
    Some(to_iso(chrono::DateTime::<Utc>::from(time)))
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

// ---------------------------------------------------------------------------
// Opening a project

/// Lets copies whose original is gone take its place, and takes in manuscript
/// and planning files whose names are not ids. True when anything changed.
pub fn reconcile(root: &Path) -> Result<bool> {
    let mut changed = false;
    for section in SECTIONS {
        let found = scan(root, section)?;
        let dir = root.join(section.dir());
        for (file, of) in &found.orphans {
            let target = dir.join(doc::file_name(of));
            if !target.exists() {
                rename(&dir.join(file), &target)?;
                changed = true;
            }
        }
        if !matches!(section, Section::Manuscript | Section::Planning) {
            continue;
        }
        for file in &found.loose {
            let stem = file.strip_suffix(".md").unwrap_or(file);
            let id = stable_id(&format!("{}/{stem}", section.dir()));
            let target = dir.join(doc::file_name(&id));
            if target.exists() || doc::locate(root, &id).is_ok() {
                // Taken in already (on this or another device): the file is
                // left as it is and shows up as a copy of that one.
                continue;
            }
            let source = dir.join(file);
            let mut d = doc::parse_doc(&read_text(&source)?, &id);
            d.meta.id = id.clone();
            if d.meta.title.trim().is_empty() {
                d.meta.title = stem.to_string();
            }
            if d.meta.created.is_empty() {
                d.meta.created = modified_iso(&source).unwrap_or_default();
            }
            doc::write_doc_file(&target, &d)?;
            fs::remove_file(&source).map_err(|e| Error::io(&source, e))?;
            changed = true;
        }
    }
    Ok(changed)
}

/// Copies of `project.json` in the project folder: `project-DESKTOP-1.json`,
/// `project (1).json` and so on, that belong to this project.
fn project_copies(root: &Path, id: Option<&str>) -> Vec<(PathBuf, Project)> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut out: Vec<(PathBuf, Project)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            name != PROJECT_FILE
                && name.starts_with("project")
                && name.ends_with(".json")
                && e.path().is_file()
        })
        .filter_map(|e| {
            let project: Project = serde_json::from_str(&read_text(&e.path()).ok()?).ok()?;
            (project.app == project::APP_NAME && id.is_none_or(|id| project.id == id))
                .then(|| (e.path(), project))
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

/// When `project.json` itself is missing but a copy of it is there, the copy
/// takes its place. True when that happened.
pub fn restore_project_file(root: &Path) -> Result<bool> {
    if root.join(PROJECT_FILE).exists() {
        return Ok(false);
    }
    let Some((path, _)) = project_copies(root, None).into_iter().next() else {
        return Ok(false);
    };
    rename(&path, &root.join(PROJECT_FILE))?;
    Ok(true)
}

/// Merges copies of `project.json` into it: parts and documents only the copy
/// has are added where the copy had them; everything else stays as it is.
/// Each merged copy is kept under `.snapshots/project/`. True when any copy
/// was merged.
pub fn merge_project_copies(root: &Path) -> Result<bool> {
    let mut current = project::load(root)?;
    let copies = project_copies(root, Some(&current.id));
    if copies.is_empty() {
        return Ok(false);
    }
    for (_, copy) in &copies {
        merge_project(&mut current, copy);
    }
    project::save(root, &current)?;
    let keep = root.join(SNAPSHOT_DIR).join("project");
    fs::create_dir_all(&keep).map_err(|e| Error::io(&keep, e))?;
    for (path, _) in &copies {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let target = keep.join(format!("{}-{name}", stamp(Utc::now())));
        rename(path, &target)?;
    }
    Ok(true)
}

/// Adds to `into` what only `other` has. `into` wins wherever both have
/// something: titles, order, settings.
pub fn merge_project(into: &mut Project, other: &Project) {
    let mut placed: HashSet<String> = project::doc_ids(into).into_iter().collect();

    for (pi, part) in other.parts.iter().enumerate() {
        match into.parts.iter().position(|p| p.id == part.id) {
            Some(i) => {
                for (di, doc) in part.docs.iter().enumerate() {
                    if placed.contains(doc) {
                        continue;
                    }
                    // Right after the chapter it followed in the copy.
                    let at = part.docs[..di]
                        .iter()
                        .rev()
                        .find_map(|prev| into.parts[i].docs.iter().position(|d| d == prev))
                        .map_or(0, |p| p + 1);
                    into.parts[i].docs.insert(at, doc.clone());
                    placed.insert(doc.clone());
                }
            }
            // A part only the copy has comes along when it brings chapters.
            None => {
                let docs: Vec<String> = part
                    .docs
                    .iter()
                    .filter(|d| placed.insert((*d).clone()))
                    .cloned()
                    .collect();
                if docs.is_empty() {
                    continue;
                }
                let at = other.parts[..pi]
                    .iter()
                    .rev()
                    .find_map(|prev| into.parts.iter().position(|p| p.id == prev.id))
                    .map_or(0, |p| p + 1);
                into.parts.insert(
                    at,
                    Part {
                        id: part.id.clone(),
                        title: part.title.clone(),
                        docs,
                    },
                );
            }
        }
    }

    for (di, doc) in other.planning.iter().enumerate() {
        if placed.contains(doc) {
            continue;
        }
        let at = other.planning[..di]
            .iter()
            .rev()
            .find_map(|prev| into.planning.iter().position(|d| d == prev))
            .map_or(0, |p| p + 1);
        into.planning.insert(at, doc.clone());
        placed.insert(doc.clone());
    }

    if let Some(types) = &other.card_types {
        let mine = into.card_types.get_or_insert_with(cards::default_types);
        for t in types {
            if !mine.iter().any(|m| m.id == t.id) {
                mine.push(t.clone());
            }
        }
    }
    for (key, value) in &other.extra {
        into.extra
            .entry(key.clone())
            .or_insert_with(|| value.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn found(file: &str, front: Option<&str>, secs: u64) -> Found {
        Found {
            file: file.into(),
            stem: file.trim_end_matches(".md").into(),
            front_id: front.map(str::to_string),
            modified: Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs)),
        }
    }

    #[test]
    fn names_from_each_sync_program() {
        let id = "k7q2m9x4t1ab";
        let files = [
            found("k7q2m9x4t1ab.md", Some(id), 1),
            found("k7q2m9x4t1ab-DESKTOP-1AB2C3D.md", Some(id), 2),
            found(
                "k7q2m9x4t1ab (홍길동's conflicted copy 2026-09-28).md",
                Some(id),
                3,
            ),
            found("k7q2m9x4t1ab 2.md", Some(id), 4),
            found("k7q2m9x4t1ab (1).md", Some(id), 5),
            found(
                "k7q2m9x4t1ab.sync-conflict-20260928-101500-ABCDEF7.md",
                Some(id),
                6,
            ),
            // Without front matter the name gives it away.
            found("k7q2m9x4t1ab - 복사본.md", None, 7),
            // Hand-made files without front matter are chapters of their own.
            found("loose.md", None, 8),
            found("메모.md", None, 9),
        ];
        let scan = classify(&files);
        assert_eq!(scan.ids, ["k7q2m9x4t1ab", "loose"]);
        assert_eq!(scan.copies.len(), 6);
        assert!(scan.copies.iter().all(|(_, of)| of == id));
        assert_eq!(scan.loose, ["메모.md"]);
        assert!(scan.orphans.is_empty());
    }

    #[test]
    fn newest_copy_takes_the_place_of_a_missing_original() {
        let files = [
            found("abc-LAPTOP.md", Some("abc"), 10),
            found("abc (1).md", Some("abc"), 20),
        ];
        let scan = classify(&files);
        assert_eq!(
            scan.orphans,
            [("abc (1).md".to_string(), "abc".to_string())]
        );
        assert_eq!(
            scan.copies,
            [("abc-LAPTOP.md".to_string(), "abc".to_string())]
        );
    }

    #[test]
    fn device_names() {
        let id = "k7q2m9x4t1ab";
        assert_eq!(
            device_of("k7q2m9x4t1ab-DESKTOP-1AB2C3D", id).as_deref(),
            Some("DESKTOP-1AB2C3D")
        );
        assert_eq!(
            device_of("k7q2m9x4t1ab-DESKTOP-1AB2C3D-2", id).as_deref(),
            Some("DESKTOP-1AB2C3D")
        );
        assert_eq!(
            device_of("k7q2m9x4t1ab (홍길동's conflicted copy 2026-09-28)", id).as_deref(),
            Some("홍길동")
        );
        assert_eq!(device_of("k7q2m9x4t1ab 2", id), None);
        assert_eq!(device_of("k7q2m9x4t1ab (1)", id), None);
    }

    fn part(id: &str, docs: &[&str]) -> Part {
        Part {
            id: id.into(),
            title: id.to_uppercase(),
            docs: docs.iter().map(|d| d.to_string()).collect(),
        }
    }

    fn project_with(parts: Vec<Part>, planning: &[&str]) -> Project {
        let mut p: Project = serde_json::from_str(
            r#"{"app":"WriterProgram","format":1,"id":"p","title":"t","kind":"webnovel","goal":{}}"#,
        )
        .unwrap();
        p.parts = parts;
        p.planning = planning.iter().map(|d| d.to_string()).collect();
        p
    }

    #[test]
    fn merging_structure_keeps_both_devices_chapters() {
        // This device added c2 after a2; the other added b1 after a1 and a
        // new part with d1, and a planning document.
        let mut mine = project_with(vec![part("a", &["a1", "a2", "c2"])], &["s"]);
        let theirs = project_with(
            vec![
                part("a", &["a1", "b1", "a2"]),
                part("d", &["d1"]),
                part("e", &[]),
            ],
            &["s", "s2"],
        );
        merge_project(&mut mine, &theirs);
        assert_eq!(
            mine.parts.len(),
            2,
            "an empty part only the copy has stays out"
        );
        assert_eq!(mine.parts[0].docs, ["a1", "b1", "a2", "c2"]);
        assert_eq!(mine.parts[1].id, "d");
        assert_eq!(mine.parts[1].docs, ["d1"]);
        assert_eq!(mine.planning, ["s", "s2"]);

        // Chapters placed differently stay where this device has them.
        let mut mine = project_with(vec![part("a", &["a1"]), part("b", &["a2"])], &[]);
        let theirs = project_with(vec![part("a", &["a1", "a2"]), part("b", &[])], &[]);
        merge_project(&mut mine, &theirs);
        assert_eq!(mine.parts[0].docs, ["a1"]);
        assert_eq!(mine.parts[1].docs, ["a2"]);
    }
}
