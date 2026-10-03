//! Daily backups of `project.json` (docs/safety-design.md S5).
//!
//! Every save also writes the same content to
//! `.backup/project-<YYYY-MM-DD>.json` (the writer's local day): one file a
//! day, the same day overwritten, the last `BACKUP_DAYS` days kept. The
//! folder travels with the project like the records, so another device can
//! restore from it too.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{Local, NaiveDate};

use super::{Project, parse};
use crate::Result;
use crate::store::atomic_write;

pub const BACKUP_DIR: &str = ".backup";
/// How many days of backups are kept.
pub const BACKUP_DAYS: usize = 7;

fn name(day: NaiveDate) -> String {
    format!("project-{}.json", day.format("%Y-%m-%d"))
}

/// The day a backup file is for; none for other files (a sync program's
/// copy of a backup, say).
fn day_of(file: &str) -> Option<NaiveDate> {
    let day = file.strip_prefix("project-")?.strip_suffix(".json")?;
    NaiveDate::parse_from_str(day, "%Y-%m-%d").ok()
}

/// Backup days on disk, newest first.
fn days(root: &Path) -> Vec<NaiveDate> {
    let Ok(entries) = fs::read_dir(root.join(BACKUP_DIR)) else {
        return Vec::new();
    };
    let mut days: Vec<NaiveDate> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| day_of(&e.file_name().to_string_lossy()))
        .collect();
    days.sort_by(|a, b| b.cmp(a));
    days
}

pub(super) fn keep(root: &Path, bytes: &[u8]) -> Result<()> {
    keep_on(root, bytes, Local::now().date_naive())
}

/// Writes `bytes` as the backup of `day` and lets go of backups beyond the
/// newest `BACKUP_DAYS` days.
pub(super) fn keep_on(root: &Path, bytes: &[u8], day: NaiveDate) -> Result<()> {
    let dir = root.join(BACKUP_DIR);
    atomic_write(&dir.join(name(day)), bytes)?;
    for old in days(root).into_iter().skip(BACKUP_DAYS) {
        // One left over is tried again next time.
        let _ = fs::remove_file(dir.join(name(old)));
    }
    Ok(())
}

/// A backup that reads as a project.
#[derive(Debug, Clone)]
pub struct Backup {
    pub day: NaiveDate,
    pub path: PathBuf,
    pub project: Project,
}

/// The newest backup that is a whole project.
pub fn latest_backup(root: &Path) -> Option<Backup> {
    days(root).into_iter().find_map(|day| {
        let path = root.join(BACKUP_DIR).join(name(day));
        let project = parse(&fs::read(&path).ok()?).ok()?;
        Some(Backup { day, path, project })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_a_day_and_seven_days_kept() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let day = |d| NaiveDate::from_ymd_opt(2026, 9, d).unwrap();
        for d in 1..=9 {
            keep_on(root, b"{}", day(d)).unwrap();
        }
        keep_on(root, b"{ }", day(9)).unwrap();
        let kept = days(root);
        assert_eq!(kept.len(), BACKUP_DAYS);
        assert_eq!(kept[0], day(9));
        assert_eq!(kept[6], day(3));
        assert_eq!(
            fs::read(root.join(BACKUP_DIR).join("project-2026-09-09.json")).unwrap(),
            b"{ }"
        );
        // Not projects: nothing to restore from.
        assert!(latest_backup(root).is_none());
        assert_eq!(day_of("project-2026-09-09 (다른 기기 t).json"), None);
    }
}
