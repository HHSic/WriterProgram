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
/// starts. The last four come from edits on two devices (see copies/):
/// this device's text that could not be saved over another device's,
/// another device's text replaced by this one's, this device's text before
/// another device's was loaded, and the text before a copy replaced it.
pub const KINDS: [&str; 9] = [
    "auto",
    "manual",
    "before-replace",
    "before-restore",
    "before-revise",
    "this-device",
    "other-device",
    "before-reload",
    "before-copy",
];

/// Kinds removed after a while, like automatic records.
const PRUNED: [&str; 2] = ["auto", "before-reload"];

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

/// Keeps a record of `doc` unless the newest record already has the same text.
pub fn keep_unless_same(root: &Path, doc: &DocFile, kind: &str) -> Result<Option<SnapshotInfo>> {
    if let Some(newest) = stems(root, &doc.meta.id)?.first()
        && load(root, &doc.meta.id, newest)?.body == doc.body
    {
        return Ok(None);
    }
    create(root, doc, kind, "").map(Some)
}

/// Every document with records, and its record stems, newest first.
fn all_stems(root: &Path) -> Result<Vec<(String, Vec<String>)>> {
    let Ok(docs) = fs::read_dir(root.join(SNAPSHOT_DIR)) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for entry in docs.filter_map(|e| e.ok()) {
        if !entry.path().is_dir() {
            continue;
        }
        let doc_id = entry.file_name().to_string_lossy().into_owned();
        let stems = stems(root, &doc_id)?;
        out.push((doc_id, stems));
    }
    Ok(out)
}

fn kind_of(stem: &str) -> &str {
    stem.rsplit('.').next().unwrap_or("")
}

fn older_than(stem: &str, cutoff: chrono::DateTime<Utc>) -> bool {
    parse_stamp(stem).is_some_and(|at| at < cutoff)
}

/// Removes automatic records (and ones kept before loading another device's
/// text) older than `keep`. Other kinds stay.
pub fn prune(root: &Path, keep: Duration) -> Result<()> {
    let cutoff = Utc::now() - keep;
    for (doc_id, stems) in all_stems(root)? {
        for stem in stems {
            if PRUNED.contains(&kind_of(&stem)) && older_than(&stem, cutoff) {
                let path = dir(root, &doc_id).join(format!("{stem}.md"));
                fs::remove_file(&path).map_err(|e| Error::io(&path, e))?;
            }
        }
    }
    Ok(())
}

/// Automatic records younger than this stay when the writer tidies records
/// (오래된 자동 기록 정리).
pub const TIDY_DAYS: i64 = 14;

/// The record files tidying removes, with their sizes: what `prune` removes
/// after `keep`, and automatic records older than [`TIDY_DAYS`] except each
/// document's newest automatic one. Records kept by hand (지금 원고 보관)
/// and the ones taken before replacing, going back, revising or another
/// device's text are left alone.
pub fn tidy_plan(root: &Path, keep: Duration) -> Result<Vec<(PathBuf, u64)>> {
    let now = Utc::now();
    let (prune_cutoff, tidy_cutoff) = (now - keep, now - Duration::days(TIDY_DAYS));
    let mut out = Vec::new();
    for (doc_id, stems) in all_stems(root)? {
        let mut newest_auto = true;
        for stem in stems {
            let kind = kind_of(&stem);
            let expired = PRUNED.contains(&kind) && older_than(&stem, prune_cutoff);
            let old_auto = kind == "auto" && !newest_auto && older_than(&stem, tidy_cutoff);
            if kind == "auto" {
                newest_auto = false;
            }
            if expired || old_auto {
                let path = dir(root, &doc_id).join(format!("{stem}.md"));
                let size = fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                out.push((path, size));
            }
        }
    }
    Ok(out)
}

/// Tidies records (see [`tidy_plan`]); returns the bytes freed.
pub fn tidy(root: &Path, keep: Duration) -> Result<u64> {
    let mut freed = 0;
    for (path, size) in tidy_plan(root, keep)? {
        match fs::remove_file(&path) {
            Ok(()) => freed += size,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(&path, e)),
        }
    }
    Ok(freed)
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
