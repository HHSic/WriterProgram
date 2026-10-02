//! Telling copies apart: which files in a section folder are originals,
//! copies, copies whose original is gone, or files of their own.

use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;

use crate::doc::{Section, decode_value};
use crate::{Error, Result};

/// Whether `s` can be a file id: letters, digits, '-' and '_' only.
pub fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// One `.md` file in a section folder.
#[derive(Debug, Clone)]
pub(super) struct Found {
    pub(super) file: String,
    pub(super) stem: String,
    /// The id in its front matter, if it has one.
    pub(super) front_id: Option<String>,
    pub(super) modified: Option<std::time::SystemTime>,
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
pub(super) fn classify(found: &[Found]) -> Scan {
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
