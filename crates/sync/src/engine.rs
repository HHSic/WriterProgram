//! One pass of keeping a project folder and its folder on a drive in step.
//!
//! Each file is compared three ways: as it is here, as it is on the drive, and
//! as both were after the last pass (`Base`):
//!
//! | here | drive | done |
//! |---|---|---|
//! | same | same | nothing |
//! | changed | same | upload (or remove there, if removed here) |
//! | same | changed | download (or remove here, if removed there) |
//! | changed | changed | same content: nothing; else keep both (below) |
//!
//! Keeping both: this device's file stays and goes up; the drive's becomes a
//! copy next to it, `<name> (다른 기기 2026-09-28 1015).md`, which the app
//! lists like the copies other sync programs leave (writer_core::copies).
//! `project.json` is merged instead (writer_core::copies::merge_project).
//! Removing on one side and changing on the other keeps the changed file.
//!
//! Writes here go around `writer_core::store::atomic_write`, so the app's
//! folder watcher sees them as changes from elsewhere, like a sync program's.
//! Each one happens under `lock` and only if the file is still as read at the
//! start of the pass; one changed in the meantime waits for the next pass.
//!
//! Nothing is removed on the strength of a file that could not be read: a
//! file here whose size, time or content could not be read, or anything in a
//! folder whose entries could not all be read, is left alone on both sides
//! for this pass (`Report::later`), and the base keeps what it knew.
//!
//! Many removals at once (a folder moved or deleted by mistake) wait for the
//! writer: when one pass would remove [`MANY_REMOVALS`] files or more, or
//! [`MANY_REMOVALS_PERCENT`]% of the files kept in step, on either side, those
//! removals are held (`Report::held_there`, `Report::held_here`) and the rest
//! of the pass goes on. The writer then has them removed or brought back
//! ([`Choices`]) in a later pass.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

use chrono::Local;
use serde::{Deserialize, Serialize};
use writer_core::copies::merge_project;
use writer_core::project::{PROJECT_FILE, Project};
use writer_core::store::{new_id, rev_of};

use crate::base::{Base, Known};
use crate::remote::{Put, Remote};
use crate::{Error, Result};

/// What a pass did, path by path.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub uploaded: Vec<String>,
    pub downloaded: Vec<String>,
    /// Removed on this device because they were removed on the drive.
    pub removed_here: Vec<String>,
    /// Removed on the drive because they were removed here.
    pub removed_there: Vec<String>,
    /// Copies made of files changed on both sides.
    pub copies: Vec<String>,
    /// `project.json` changed on both sides and was merged.
    pub merged: bool,
    /// Changed again during the pass, or not readable here right now; taken
    /// up by the next one.
    pub later: Vec<String>,
    /// Removed here, but too many at once: removing them on the drive too
    /// waits for the writer ([`Choices`]).
    pub held_there: Vec<String>,
    /// Removed on the drive, but too many at once: removing them here too
    /// waits for the writer.
    pub held_here: Vec<String>,
    /// Room left on the drive in bytes, when it is running low; none when
    /// there is room or the drive does not say (`accounts::sync_project`).
    pub space_left: Option<u64>,
}

/// What the writer said about removals an earlier pass held back.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choices {
    /// Paths to remove even though many go at once.
    #[serde(default)]
    pub remove: BTreeSet<String>,
    /// Paths not to remove after all: they come back from the side that
    /// still has them.
    #[serde(default)]
    pub keep: BTreeSet<String>,
}

/// One pass removing this many files on one side waits for the writer …
pub const MANY_REMOVALS: usize = 20;
/// … and so does removing this share (in percent) of the files kept in step,
/// once it is more than a handful ([`FEW_REMOVALS`]).
pub const MANY_REMOVALS_PERCENT: usize = 30;
/// Removing fewer files than this never waits for the writer, however small
/// the project.
pub const FEW_REMOVALS: usize = 5;

/// Whether removing `count` files of the `synced` kept in step is too many
/// to do without asking.
pub fn too_many_removals(count: usize, synced: usize) -> bool {
    count >= MANY_REMOVALS
        || (count >= FEW_REMOVALS && count * 100 >= synced * MANY_REMOVALS_PERCENT)
}

/// A file here: its fingerprint, its size and time, and its content when read.
struct Here {
    rev: String,
    stamp: Option<(u64, i64)>,
}

/// The project folder here, as far as it could be read.
#[derive(Default)]
struct Scan {
    files: BTreeMap<String, Here>,
    /// Files whose size, time or content could not be read this time.
    unknown: BTreeSet<String>,
    /// Folders whose entries could not all be read; nothing in them is known.
    unknown_dirs: Vec<String>,
}

impl Scan {
    /// Whether this pass knows how `rel` is here.
    fn knows(&self, rel: &str) -> bool {
        !self.unknown.contains(rel)
            && !self.unknown_dirs.iter().any(|dir| {
                rel.strip_prefix(dir.as_str())
                    .is_some_and(|rest| rest.starts_with('/'))
            })
    }
}

/// Hidden folders that travel with the project: records, trash, the
/// creation journal (files per device and month, so both sides never change
/// the same one, and past months never change again), what was sent to
/// editors (교정본 주고받기) and the daily backups of project.json.
const HIDDEN_KEPT: [&str; 5] = [".snapshots", ".trash", ".journal", ".exchanges", ".backup"];

/// Which files belong to the project on a drive: everything but temporary
/// files, hidden folders other than the kept ones, and system files.
fn wanted(rel: &str) -> bool {
    let parts: Vec<&str> = rel.split('/').collect();
    let (name, dirs) = parts.split_last().expect("at least a name");
    let hidden_dir = dirs
        .iter()
        .any(|d| d.starts_with('.') && !HIDDEN_KEPT.contains(d));
    !hidden_dir
        && !name.starts_with('.')
        && !name.starts_with("~$")
        && !name.eq_ignore_ascii_case("desktop.ini")
        && !name.eq_ignore_ascii_case("thumbs.db")
        && !name.ends_with(".tmp")
}

fn stamp_of(meta: &fs::Metadata) -> Option<(u64, i64)> {
    let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some((meta.len(), modified.as_nanos() as i64))
}

fn rel_of(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("inside root")
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Reads the folder `dir` into `out`. The project folder itself must be
/// readable; a folder inside it that is not, or an entry that is not, is
/// noted as unknown rather than taken for gone.
fn scan(root: &Path, dir: &Path, base: &Base, out: &mut Scan) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if dir == root => return Err(Error::io(dir, e)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => {
            out.unknown_dirs.push(rel_of(root, dir));
            return Ok(());
        }
    };
    for entry in entries {
        let Ok(entry) = entry else {
            let rel = rel_of(root, dir);
            if rel.is_empty() {
                return Err(Error::Invalid(
                    "작품 폴더의 파일 목록을 다 읽지 못함".into(),
                ));
            }
            out.unknown_dirs.push(rel);
            continue;
        };
        let path = entry.path();
        let rel = rel_of(root, &path);
        let meta = match entry.metadata() {
            Ok(meta) => meta,
            // Gone since the folder was listed.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(_) => {
                // A file or a folder: either way, nothing under this name is known.
                if wanted(&rel) {
                    out.unknown.insert(rel.clone());
                }
                out.unknown_dirs.push(rel);
                continue;
            }
        };
        if meta.is_dir() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with('.') || HIDDEN_KEPT.contains(&name.as_str()) {
                scan(root, &path, base, out)?;
            }
            continue;
        }
        if !wanted(&rel) {
            continue;
        }
        let stamp = stamp_of(&meta);
        // Unchanged size and time: the fingerprint from last time holds.
        let rev = match base.files.get(&rel) {
            Some(known) if known.stamp.is_some() && known.stamp == stamp => known.local.clone(),
            _ => match fs::read(&path) {
                Ok(bytes) => rev_of(&bytes),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                // Held by another program or being written right now.
                Err(_) => {
                    out.unknown.insert(rel);
                    continue;
                }
            },
        };
        out.files.insert(rel, Here { rev, stamp });
    }
    Ok(())
}

/// The fingerprint of a file here, none when it is not there. An error when
/// it is there but cannot be read.
fn local_rev(path: &Path) -> std::io::Result<Option<String>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(rev_of(&bytes))),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

/// Writes a file here the way a sync program would: complete or not at all.
fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| Error::Invalid("저장 위치가 올바르지 않음".into()))?;
    fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = dir.join(format!(".{name}.{}.sync.tmp", new_id()));
    let written = (|| {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()
    })();
    if let Err(e) = written.and_then(|()| fs::rename(&tmp, path)) {
        let _ = fs::remove_file(&tmp);
        return Err(Error::io(path, e));
    }
    Ok(())
}

/// `manuscript/abc.md` → `manuscript/abc (다른 기기 2026-09-28 1015).md`.
pub fn copy_name(rel: &str, when: &str) -> String {
    let (dir, name) = match rel.rsplit_once('/') {
        Some((dir, name)) => (format!("{dir}/"), name),
        None => (String::new(), rel),
    };
    let (stem, ext) = match name.rsplit_once('.') {
        Some((stem, ext)) if !stem.is_empty() => (stem, format!(".{ext}")),
        _ => (name, String::new()),
    };
    format!("{dir}{stem} (다른 기기 {when}){ext}")
}

struct Pass<'a> {
    root: &'a Path,
    remote: &'a mut dyn Remote,
    lock: &'a Mutex<()>,
    report: Report,
}

impl Pass<'_> {
    fn full(&self, rel: &str) -> PathBuf {
        rel.split('/')
            .fold(self.root.to_path_buf(), |p, part| p.join(part))
    }

    /// Writes `bytes` here if the file is still as read (`expect`, none for
    /// "not there"). False when it changed in the meantime.
    fn write_here(&mut self, rel: &str, bytes: &[u8], expect: Option<&str>) -> Result<bool> {
        let path = self.full(rel);
        let _guard = self.lock.lock().unwrap_or_else(|p| p.into_inner());
        match local_rev(&path) {
            Ok(rev) if rev.as_deref() == expect => {}
            _ => return Ok(false),
        }
        write_file(&path, bytes)?;
        Ok(true)
    }

    /// Removes a file here if it is still as read. False when it changed in
    /// the meantime or cannot be read or removed right now.
    fn remove_here(&mut self, rel: &str, expect: &str) -> bool {
        let path = self.full(rel);
        let _guard = self.lock.lock().unwrap_or_else(|p| p.into_inner());
        match local_rev(&path) {
            Ok(Some(rev)) if rev == expect => fs::remove_file(&path).is_ok(),
            Ok(None) => true,
            _ => false,
        }
    }

    fn read_here(&self, rel: &str) -> Result<Vec<u8>> {
        let path = self.full(rel);
        fs::read(&path).map_err(|e| Error::io(&path, e))
    }

    fn upload(&mut self, base: &mut Base, rel: &str, expected: Option<&str>) -> Result<()> {
        let Ok(bytes) = self.read_here(rel) else {
            self.report.later.push(rel.into());
            return Ok(());
        };
        match self.remote.put(rel, &bytes, expected)? {
            Put::Done(rev) => {
                let stamp = fs::metadata(self.full(rel)).ok().and_then(|m| stamp_of(&m));
                base.files.insert(
                    rel.into(),
                    Known {
                        local: rev_of(&bytes),
                        remote: rev,
                        stamp,
                    },
                );
                self.report.uploaded.push(rel.into());
            }
            Put::Changed => self.report.later.push(rel.into()),
        }
        Ok(())
    }

    fn download(
        &mut self,
        base: &mut Base,
        rel: &str,
        rev: &str,
        expect_here: Option<&str>,
    ) -> Result<()> {
        let bytes = self.remote.get(rel)?;
        if !self.write_here(rel, &bytes, expect_here)? {
            self.report.later.push(rel.into());
            return Ok(());
        }
        let stamp = fs::metadata(self.full(rel)).ok().and_then(|m| stamp_of(&m));
        base.files.insert(
            rel.into(),
            Known {
                local: rev_of(&bytes),
                remote: rev.into(),
                stamp,
            },
        );
        self.report.downloaded.push(rel.into());
        Ok(())
    }

    /// Changed on both sides.
    fn both(&mut self, base: &mut Base, rel: &str, here: &Here, rev: &str) -> Result<()> {
        let theirs = self.remote.get(rel)?;
        if rev_of(&theirs) == here.rev {
            // The same change on both sides.
            base.files.insert(
                rel.into(),
                Known {
                    local: here.rev.clone(),
                    remote: rev.into(),
                    stamp: here.stamp,
                },
            );
            return Ok(());
        }
        if rel == PROJECT_FILE {
            return self.merge_structure(base, here, rev, &theirs);
        }
        // Keep both: the drive's version becomes a copy here and there.
        let when = Local::now().format("%Y-%m-%d %H%M").to_string();
        let copy = copy_name(rel, &when);
        if !self.write_here(&copy, &theirs, None)? {
            self.report.later.push(rel.into());
            return Ok(());
        }
        self.report.copies.push(copy.clone());
        if let Put::Done(copy_rev) = self.remote.put(&copy, &theirs, None)? {
            base.files.insert(
                copy.clone(),
                Known {
                    local: rev_of(&theirs),
                    remote: copy_rev,
                    stamp: fs::metadata(self.full(&copy))
                        .ok()
                        .and_then(|m| stamp_of(&m)),
                },
            );
        }
        self.upload(base, rel, Some(rev))
    }

    fn merge_structure(
        &mut self,
        base: &mut Base,
        here: &Here,
        rev: &str,
        theirs: &[u8],
    ) -> Result<()> {
        let mine_bytes = self.read_here(PROJECT_FILE)?;
        let parse = |bytes: &[u8]| serde_json::from_slice::<Project>(bytes).ok();
        let (Some(mut mine), Some(other)) = (parse(&mine_bytes), parse(theirs)) else {
            // One side is not readable: keep this device's, as for any file.
            return self.upload(base, PROJECT_FILE, Some(rev));
        };
        merge_project(&mut mine, &other);
        let mut merged = serde_json::to_string_pretty(&mine).expect("project serializes");
        merged.push('\n');
        if !self.write_here(PROJECT_FILE, merged.as_bytes(), Some(&here.rev))? {
            self.report.later.push(PROJECT_FILE.into());
            return Ok(());
        }
        self.report.merged = true;
        self.upload(base, PROJECT_FILE, Some(rev))
    }
}

/// One pass over the project folder at `root` and its folder on the drive.
/// `lock` is held around each write here, so saves from the editor never
/// meet a download half way. `choices` carries what the writer said about
/// removals an earlier pass held back (none on a plain pass).
pub fn sync(
    root: &Path,
    remote: &mut dyn Remote,
    base: &mut Base,
    lock: &Mutex<()>,
    choices: &Choices,
) -> Result<Report> {
    // Kept after all: forgetting them makes the side that has them win.
    for rel in &choices.keep {
        base.files.remove(rel);
    }
    let synced = base.files.len();
    let mut scanned = Scan::default();
    scan(root, root, base, &mut scanned)?;
    let here = &scanned.files;
    let there: BTreeMap<String, String> = remote
        .list()?
        .into_iter()
        .filter(|f| crate::remote::safe_path(&f.path) && wanted(&f.path))
        .map(|f| (f.path, f.rev))
        .collect();
    let paths: BTreeSet<String> = here
        .keys()
        .chain(there.keys())
        .chain(base.files.keys())
        .chain(scanned.unknown.iter())
        .cloned()
        .collect();

    let mut pass = Pass {
        root,
        remote,
        lock,
        report: Report::default(),
    };
    // Removals wait until everything else is done, to be counted first.
    let mut remove_there: Vec<(String, String)> = Vec::new();
    let mut remove_here: Vec<(String, String)> = Vec::new();
    for rel in paths {
        if !scanned.knows(&rel) {
            // Not readable here right now: nothing goes either way.
            pass.report.later.push(rel);
            continue;
        }
        let (l, r, b) = (
            here.get(&rel),
            there.get(&rel),
            base.files.get(&rel).cloned(),
        );
        let changed_here = l.map(|h| &h.rev) != b.as_ref().map(|k| &k.local);
        let changed_there = r != b.as_ref().map(|k| &k.remote);
        match (changed_here, changed_there, l, r) {
            (false, false, Some(l), _) => {
                // Remember the size and time, to skip reading it next time.
                if let Some(known) = base.files.get_mut(&rel) {
                    known.stamp = l.stamp;
                }
            }
            (false, false, None, _) => {}
            // Changed here only.
            (true, false, Some(_), r) => pass.upload(base, &rel, r.map(String::as_str))?,
            (true, false, None, Some(r)) => remove_there.push((rel, r.clone())),
            (true, false, None, None) => {
                base.files.remove(&rel);
            }
            // Changed on the drive only.
            (false, true, l, Some(r)) => {
                let expect = l.map(|h| h.rev.clone());
                pass.download(base, &rel, r, expect.as_deref())?;
            }
            (false, true, Some(l), None) => remove_here.push((rel, l.rev.clone())),
            (false, true, None, None) => {
                base.files.remove(&rel);
            }
            // Changed on both sides.
            (true, true, Some(l), Some(r)) => pass.both(base, &rel, l, r)?,
            // Removed on one side, changed on the other: the change stays.
            (true, true, Some(_), None) => pass.upload(base, &rel, None)?,
            (true, true, None, Some(r)) => pass.download(base, &rel, r, None)?,
            (true, true, None, None) => {
                base.files.remove(&rel);
            }
        }
    }

    let (remove_there, held) = hold_many(remove_there, synced, choices);
    pass.report.held_there = held;
    for (rel, r) in remove_there {
        if pass.remote.remove(&rel, &r)? {
            base.files.remove(&rel);
            pass.report.removed_there.push(rel);
        } else {
            pass.report.later.push(rel);
        }
    }
    let (remove_here, held) = hold_many(remove_here, synced, choices);
    pass.report.held_here = held;
    for (rel, rev) in remove_here {
        if pass.remove_here(&rel, &rev) {
            base.files.remove(&rel);
            pass.report.removed_here.push(rel);
        } else {
            pass.report.later.push(rel);
        }
    }
    base.synced_at = Some(writer_core::store::now_iso());
    Ok(pass.report)
}

/// Splits planned removals into the ones to do now and the ones held for the
/// writer: when too many were not confirmed yet, only the confirmed ones go.
fn hold_many(
    planned: Vec<(String, String)>,
    synced: usize,
    choices: &Choices,
) -> (Vec<(String, String)>, Vec<String>) {
    let unconfirmed = planned
        .iter()
        .filter(|(rel, _)| !choices.remove.contains(rel))
        .count();
    if !too_many_removals(unconfirmed, synced) {
        return (planned, Vec::new());
    }
    let (go, held): (Vec<_>, Vec<_>) = planned
        .into_iter()
        .partition(|(rel, _)| choices.remove.contains(rel));
    (go, held.into_iter().map(|(rel, _)| rel).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_names() {
        assert_eq!(
            copy_name("manuscript/abc.md", "2026-09-28 1015"),
            "manuscript/abc (다른 기기 2026-09-28 1015).md"
        );
        assert_eq!(copy_name("notes", "t"), "notes (다른 기기 t)");
        assert_eq!(
            copy_name(".trash/x/item.json", "t"),
            ".trash/x/item (다른 기기 t).json"
        );
    }

    #[test]
    fn many_removals() {
        assert!(!too_many_removals(19, 1000));
        assert!(too_many_removals(20, 1000));
        assert!(too_many_removals(30, 100));
        assert!(too_many_removals(5, 10));
        assert!(!too_many_removals(5, 20));
        // A handful in a small project goes without asking.
        assert!(!too_many_removals(4, 5));
    }

    #[test]
    fn what_a_pass_knows() {
        let mut scan = Scan::default();
        scan.unknown.insert("manuscript/a.md".into());
        scan.unknown_dirs.push(".snapshots/x".into());
        assert!(!scan.knows("manuscript/a.md"));
        assert!(scan.knows("manuscript/b.md"));
        assert!(!scan.knows(".snapshots/x/1.auto.md"));
        assert!(scan.knows(".snapshots/xy/1.auto.md"));
    }

    #[test]
    fn which_files_travel() {
        assert!(wanted("manuscript/abc.md"));
        assert!(wanted(".snapshots/abc/20260928-101500-000.auto.md"));
        assert!(wanted(".exchanges/abc/exchange.json"));
        assert!(wanted(".trash/x/item.json"));
        assert!(wanted(".journal/k7q2m9x4t1ab.jsonl"));
        assert!(wanted(".journal/k7q2m9x4t1ab/2026-10.jsonl"));
        assert!(wanted(".backup/project-2026-10-03.json"));
        assert!(!wanted("manuscript/.abc.md.x1.tmp"));
        assert!(!wanted(".git/config"));
        assert!(!wanted("desktop.ini"));
        assert!(!wanted("manuscript/~$abc.docx"));
    }
}
