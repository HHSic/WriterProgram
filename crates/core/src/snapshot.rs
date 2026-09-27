//! Records (기록): copies of a document at points in time.
//!
//! Each record is a document file under `.snapshots/<doc id>/`, named
//! `<UTC stamp>.<kind>.md` so names sort by time. The front matter carries
//! `snapshotKind`, `snapshotName` and `snapshotAt`.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Duration, Utc};
use serde::Serialize;

use crate::count::{Counts, count_blocks};
use crate::doc::{self, DocFile};
use crate::store::{parse_stamp, stamp, to_iso};
use crate::{Error, Result};

pub const SNAPSHOT_DIR: &str = ".snapshots";

/// Kinds of records: automatic, kept by hand ("지금 원고 보관"), and the ones
/// taken before replace-all, before going back to a record, and when revising
/// starts.
pub const KINDS: [&str; 5] = [
    "auto",
    "manual",
    "before-replace",
    "before-restore",
    "before-revise",
];

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotInfo {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub at: String,
    pub counts: Counts,
}

fn dir(root: &Path, doc_id: &str) -> PathBuf {
    root.join(SNAPSHOT_DIR).join(doc_id)
}

fn check_id(id: &str) -> Result<()> {
    let ok = !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.' || c == '_')
        && !id.contains("..");
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid("올바르지 않은 기록 이름".into()))
    }
}

/// Keeps a record of `doc` as it is now.
pub fn create(root: &Path, doc: &DocFile, kind: &str, name: &str) -> Result<SnapshotInfo> {
    if !KINDS.contains(&kind) {
        return Err(Error::Invalid(format!("알 수 없는 기록 종류: {kind}")));
    }
    let dir = dir(root, &doc.meta.id);
    let mut at = Utc::now();
    let (id, path) = loop {
        let id = format!("{}.{kind}", stamp(at));
        let path = dir.join(format!("{id}.md"));
        if !path.exists() {
            break (id, path);
        }
        at += Duration::milliseconds(1);
    };
    let mut copy = doc.clone();
    copy.meta.set_extra_str("snapshotKind", kind);
    copy.meta.set_extra_str("snapshotName", name.trim());
    copy.meta.set_extra_str("snapshotAt", &to_iso(at));
    doc::write_doc_file(&path, &copy)?;
    Ok(SnapshotInfo {
        id,
        kind: kind.into(),
        name: name.trim().into(),
        at: to_iso(at),
        counts: count_blocks(&doc.body),
    })
}

/// Record file stems of a document, newest first.
fn stems(root: &Path, doc_id: &str) -> Result<Vec<String>> {
    let dir = dir(root, doc_id);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(&dir, e)),
    };
    let mut stems: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with('.'))
        .filter_map(|n| n.strip_suffix(".md").map(str::to_string))
        .collect();
    stems.sort_unstable_by(|a, b| b.cmp(a));
    Ok(stems)
}

fn info(root: &Path, doc_id: &str, id: &str) -> Result<SnapshotInfo> {
    let file = load(root, doc_id, id)?;
    let kind = id.rsplit('.').next().unwrap_or("auto").to_string();
    let at = file
        .meta
        .extra_str("snapshotAt")
        .or_else(|| parse_stamp(id).map(to_iso))
        .unwrap_or_default();
    Ok(SnapshotInfo {
        id: id.into(),
        kind,
        name: file.meta.extra_str("snapshotName").unwrap_or_default(),
        at,
        counts: count_blocks(&file.body),
    })
}

/// Records of a document, newest first.
pub fn list(root: &Path, doc_id: &str) -> Result<Vec<SnapshotInfo>> {
    stems(root, doc_id)?
        .iter()
        .map(|id| info(root, doc_id, id))
        .collect()
}

pub fn load(root: &Path, doc_id: &str, id: &str) -> Result<DocFile> {
    check_id(doc_id)?;
    check_id(id)?;
    let path = dir(root, doc_id).join(format!("{id}.md"));
    if !path.is_file() {
        return Err(Error::NotFound("기록을 찾을 수 없음".into()));
    }
    doc::read_doc(&path)
}

/// Puts a record's text back into the document. The text as it was just
/// before is kept as a "before-restore" record, which is returned.
pub fn restore(root: &Path, doc_id: &str, id: &str) -> Result<SnapshotInfo> {
    let (_, path) = doc::locate(root, doc_id)?;
    let current = doc::read_doc(&path)?;
    let record = load(root, doc_id, id)?;
    let before = create(root, &current, "before-restore", "")?;
    doc::write_doc_file(
        &path,
        &DocFile {
            meta: current.meta,
            body: record.body,
        },
    )?;
    Ok(before)
}

/// Keeps an automatic record of `current` (the text about to be replaced) when
/// the newest record is older than `every` and differs from it.
pub fn auto_if_due(
    root: &Path,
    current: &DocFile,
    every: Duration,
) -> Result<Option<SnapshotInfo>> {
    if count_blocks(&current.body).with_spaces == 0 {
        return Ok(None);
    }
    let doc_id = &current.meta.id;
    if let Some(newest) = stems(root, doc_id)?.first() {
        if let Some(at) = parse_stamp(newest)
            && Utc::now() - at < every
        {
            return Ok(None);
        }
        if load(root, doc_id, newest)?.body == current.body {
            return Ok(None);
        }
    }
    create(root, current, "auto", "").map(Some)
}

/// Removes automatic records older than `keep`. Other kinds stay.
pub fn prune(root: &Path, keep: Duration) -> Result<()> {
    let base = root.join(SNAPSHOT_DIR);
    let Ok(docs) = fs::read_dir(&base) else {
        return Ok(());
    };
    let cutoff = Utc::now() - keep;
    for entry in docs.filter_map(|e| e.ok()) {
        let doc_id = entry.file_name().to_string_lossy().into_owned();
        for stem in stems(root, &doc_id)? {
            if !stem.ends_with(".auto") {
                continue;
            }
            if parse_stamp(&stem).is_some_and(|at| at < cutoff) {
                let path = dir(root, &doc_id).join(format!("{stem}.md"));
                fs::remove_file(&path).map_err(|e| Error::io(&path, e))?;
            }
        }
    }
    Ok(())
}

/// Deletes every record of a document (used when it is deleted for good).
pub fn remove_all(root: &Path, doc_id: &str) -> Result<()> {
    check_id(doc_id)?;
    let dir = dir(root, doc_id);
    match fs::remove_dir_all(&dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(Error::io(&dir, e)),
    }
}
