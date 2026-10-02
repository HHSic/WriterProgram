//! How much room a project takes, by kind of file (이 작품 크기), and how much
//! room is left on the disk it is on.
//!
//! Records can grow far past the manuscript: 200 chapters with 50 records
//! each is over 100 MB, while the text itself is a few MB. When they do, the
//! app suggests tidying old automatic records (snapshot::tidy).

use std::fs;
use std::path::Path;

use chrono::Duration;
use serde::Serialize;

use super::{AUTO_RECORD_DAYS, MANUSCRIPT_DIR, PLANNING_DIR, PROJECT_FILE, load};
use crate::Result;
use crate::cards::CARDS_DIR;
use crate::notes::NOTES_DIR;
use crate::snapshot::{self, SNAPSHOT_DIR};
use crate::trash::TRASH_DIR;

/// The writing journal's folder, when the project has one.
pub const JOURNAL_DIR: &str = ".journal";

const MB: u64 = 1024 * 1024;

/// Sizes in bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Sizes {
    /// The writing itself: project.json, chapters, planning, cards and notes.
    pub writing: u64,
    /// Records (.snapshots).
    pub records: u64,
    /// Of the records, the ones kept only as each chapter's last state of a
    /// day (창작 과정 보관); 0 while that is off.
    pub daily: u64,
    pub trash: u64,
    /// The writing journal (.journal); 0 when there is none.
    pub journal: u64,
    /// What was sent to editors and the corrected files (.exchanges).
    pub exchanges: u64,
    /// What tidying old automatic records would free.
    pub tidy_frees: u64,
    /// Records have grown much bigger than the writing, and tidying would
    /// free something: the app suggests it.
    pub suggest_tidy: bool,
    /// Free room on the disk the project is on; none when unknown.
    pub disk_free: Option<u64>,
}

impl Sizes {
    /// What goes to a drive when the project is kept in step with one: all
    /// of it (the journal and the exchanges travel too, crates/sync engine).
    pub fn travelling(&self) -> u64 {
        self.writing + self.records + self.trash + self.journal + self.exchanges
    }
}

/// Records worth tidying: over 20 MB and over ten times the writing.
fn crowded(writing: u64, records: u64) -> bool {
    records > 20 * MB && records > writing.saturating_mul(10)
}

/// Bytes in a file or folder, all the way down; what cannot be read counts 0.
fn size_of(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if !meta.is_dir() {
        return meta.len();
    }
    fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| size_of(&e.path()))
                .sum()
        })
        .unwrap_or(0)
}

pub fn sizes(root: &Path) -> Result<Sizes> {
    let writing = [
        PROJECT_FILE,
        MANUSCRIPT_DIR,
        PLANNING_DIR,
        CARDS_DIR,
        NOTES_DIR,
    ]
    .iter()
    .map(|p| size_of(&root.join(p)))
    .sum();
    let records = size_of(&root.join(SNAPSHOT_DIR));
    let keep = Duration::days(AUTO_RECORD_DAYS);
    let keep_daily = load(root).is_ok_and(|p| p.keep_daily);
    let tidy_frees = snapshot::tidy_plan(root, keep, keep_daily)?
        .iter()
        .map(|(_, size)| size)
        .sum();
    let daily = if keep_daily {
        snapshot::daily_size(root, keep)?
    } else {
        0
    };
    Ok(Sizes {
        writing,
        records,
        daily,
        trash: size_of(&root.join(TRASH_DIR)),
        journal: size_of(&root.join(JOURNAL_DIR)),
        exchanges: size_of(&root.join(crate::corrections::EXCHANGE_DIR)),
        tidy_frees,
        suggest_tidy: crowded(writing, records) && tidy_frees > 0,
        disk_free: disk_free(root),
    })
}

/// Tidies old automatic records (snapshot::tidy_plan); returns the bytes freed.
pub fn tidy_records(root: &Path) -> Result<u64> {
    let keep_daily = load(root)?.keep_daily;
    snapshot::tidy(root, Duration::days(AUTO_RECORD_DAYS), keep_daily)
}

/// Free room for this user on the disk holding `path`.
#[cfg(windows)]
pub fn disk_free(path: &Path) -> Option<u64> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let mut free = 0u64;
    let ok = unsafe {
        GetDiskFreeSpaceExW(
            wide.as_ptr(),
            &mut free,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    (ok != 0).then_some(free)
}

/// Free room for this user on the disk holding `path`.
#[cfg(unix)]
pub fn disk_free(path: &Path) -> Option<u64> {
    use std::os::unix::ffi::OsStrExt;
    let c = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c.as_ptr(), &mut stat) } != 0 {
        return None;
    }
    #[allow(clippy::unnecessary_cast)]
    Some(stat.f_bavail as u64 * stat.f_frsize as u64)
}

#[cfg(not(any(windows, unix)))]
pub fn disk_free(_path: &Path) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_crowd_when_far_bigger_than_the_writing() {
        assert!(!crowded(3 * MB, 19 * MB));
        assert!(!crowded(3 * MB, 25 * MB));
        assert!(crowded(2 * MB, 25 * MB));
        assert!(crowded(0, 21 * MB));
    }

    #[test]
    fn the_disk_says_how_much_room_is_left() {
        let dir = std::env::temp_dir();
        assert!(disk_free(&dir).is_some_and(|free| free > 0));
    }
}
