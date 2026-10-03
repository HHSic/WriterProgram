//! File primitives: atomic writes, reading text, ids and timestamps.

use std::collections::hash_map::RandomState;
use std::fs::{self, File};
use std::hash::{BuildHasher, Hasher};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, SecondsFormat, Utc};
use sha2::{Digest, Sha256};

use crate::{Error, Result};

type WriteHook = Box<dyn Fn(&Path, &[u8]) + Send + Sync>;

static WRITE_HOOK: OnceLock<WriteHook> = OnceLock::new();

/// Registers a function called after every successful `atomic_write`, so the
/// app can tell its own writes from changes made by other programs (a sync
/// client bringing another device's edits). Only the first call counts.
pub fn on_write(hook: impl Fn(&Path, &[u8]) + Send + Sync + 'static) {
    let _ = WRITE_HOOK.set(Box::new(hook));
}

/// SHA-256 of some content.
pub fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// Short fingerprint of some content: the first 16 hex digits of its SHA-256.
/// The same on every device, so it can also compare files across devices.
pub fn rev_of(bytes: &[u8]) -> String {
    sha256(bytes)[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Writes `bytes` to `path` so that the file is either the old or the new
/// content, never a mix: write a temporary file next to it, flush it to disk,
/// then rename it over the target.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let dir = path
        .parent()
        .ok_or_else(|| Error::Invalid(format!("저장 위치가 올바르지 않음: {}", path.display())))?;
    fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let tmp = dir.join(format!(".{name}.{}.tmp", new_id()));

    let written = (|| {
        let mut file = File::create(&tmp)?;
        file.write_all(bytes)?;
        file.sync_all()
    })();
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return Err(Error::io(path, e));
    }

    if let Err(e) = rename_retry(&tmp, path, 6, Duration::from_millis(40)) {
        let _ = fs::remove_file(&tmp);
        return Err(Error::io(path, e));
    }

    #[cfg(unix)]
    if let Ok(d) = File::open(dir) {
        let _ = d.sync_all();
    }
    if let Some(hook) = WRITE_HOOK.get() {
        hook(path, bytes);
    }
    Ok(())
}

/// Renames `from` to `to`, trying up to `tries` times. Sync clients and virus
/// scanners briefly lock files on Windows, which makes a rename fail with
/// "access denied"; waiting a little longer after each try (`wait`, then twice
/// that, …) rides that out. Other errors are returned at once.
pub(crate) fn rename_retry(
    from: &Path,
    to: &Path,
    tries: u32,
    wait: Duration,
) -> std::io::Result<()> {
    let mut last = None;
    for attempt in 0..tries {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                last = Some(e);
                thread::sleep(wait * (attempt + 1));
            }
            Err(e) => return Err(e),
        }
    }
    Err(last.expect("retried at least once"))
}

/// Removes a file the way [`rename_retry`] renames one: a brief lock by a sync
/// client or virus scanner ("access denied") is waited out. A file that is
/// already gone counts as removed.
pub(crate) fn remove_file_retry(path: &Path, tries: u32, wait: Duration) -> std::io::Result<()> {
    retry_removal(|| fs::remove_file(path), tries, wait)
}

/// Removes a folder and everything in it, like [`remove_file_retry`].
pub(crate) fn remove_dir_all_retry(path: &Path, tries: u32, wait: Duration) -> std::io::Result<()> {
    retry_removal(|| fs::remove_dir_all(path), tries, wait)
}

fn retry_removal(
    remove: impl Fn() -> std::io::Result<()>,
    tries: u32,
    wait: Duration,
) -> std::io::Result<()> {
    let mut last = None;
    for attempt in 0..tries {
        match remove() {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                last = Some(e);
                thread::sleep(wait * (attempt + 1));
            }
            Err(e) => return Err(e),
        }
    }
    Err(last.expect("retried at least once"))
}

/// What a clean-up pass (expired trash, old automatic records) did. What
/// could not be removed is skipped and tried again the next time a project
/// opens; it never stops the project from opening.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Cleanup {
    pub removed: usize,
    pub failed: Vec<PathBuf>,
}

impl Cleanup {
    pub fn add(&mut self, other: Cleanup) {
        self.removed += other.removed;
        self.failed.extend(other.failed);
    }
}

/// Reads a UTF-8 text file, dropping a byte order mark and turning CRLF into LF.
pub fn read_text(path: &Path) -> Result<String> {
    let bytes = fs::read(path).map_err(|e| Error::io(path, e))?;
    let text = String::from_utf8(bytes).map_err(|_| Error::format(path, "UTF-8 글자가 아님"))?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    Ok(text.replace("\r\n", "\n"))
}

static COUNTER: AtomicU64 = AtomicU64::new(0);

/// A short random id: 12 characters of lowercase Crockford base32 (60 bits).
pub fn new_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(nanos);
    hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    hasher.write_u32(std::process::id());
    id_from(hasher.finish())
}

/// An id worked out from `seed`: the same seed gives the same id on every
/// device (used for files taken into a project under a name of their own).
pub fn stable_id(seed: &str) -> String {
    let digest = Sha256::digest(seed.as_bytes());
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&digest[..8]);
    id_from(u64::from_le_bytes(bytes))
}

fn id_from(mut v: u64) -> String {
    const ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";
    let mut id = String::with_capacity(12);
    for _ in 0..12 {
        id.push(ALPHABET[(v & 31) as usize] as char);
        v >>= 5;
    }
    id
}

/// Current time as an RFC 3339 string in UTC with milliseconds.
pub fn now_iso() -> String {
    to_iso(Utc::now())
}

pub fn to_iso(t: DateTime<Utc>) -> String {
    t.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// When a file was last written, as an RFC 3339 string; none when unknown.
pub(crate) fn modified_iso(path: &Path) -> Option<String> {
    let time = fs::metadata(path).ok()?.modified().ok()?;
    Some(to_iso(DateTime::<Utc>::from(time)))
}

pub fn parse_iso(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

/// Compact UTC stamp for file names that sorts by time: `20260927-101500-123`.
pub fn stamp(t: DateTime<Utc>) -> String {
    t.format("%Y%m%d-%H%M%S-%3f").to_string()
}

pub fn parse_stamp(s: &str) -> Option<DateTime<Utc>> {
    let naive = chrono::NaiveDateTime::parse_from_str(s.get(..15)?, "%Y%m%d-%H%M%S").ok()?;
    let millis: i64 = s.get(16..19)?.parse().ok()?;
    Some(naive.and_utc() + chrono::Duration::milliseconds(millis))
}

/// Turns a title into a safe folder or file name on Windows, macOS and Linux.
pub fn safe_file_name(title: &str, fallback: &str) -> String {
    let mut name: String = title
        .chars()
        .map(|c| match c {
            '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    name = name.trim().trim_end_matches('.').trim().to_string();
    if name.chars().count() > 60 {
        name = name.chars().take(60).collect::<String>().trim().to_string();
    }
    let upper = name.to_ascii_uppercase();
    let base = upper.split('.').next().unwrap_or("");
    let reserved = matches!(base, "CON" | "PRN" | "AUX" | "NUL")
        || ((base.starts_with("COM") || base.starts_with("LPT"))
            && base.len() == 4
            && base.as_bytes()[3].is_ascii_digit());
    if reserved {
        name.push('_');
    }
    if name.is_empty() {
        fallback.to_string()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_short_and_distinct() {
        let a = new_id();
        let b = new_id();
        assert_eq!(a.len(), 12);
        assert_ne!(a, b);
    }

    #[test]
    fn stable_ids_and_revs() {
        assert_eq!(stable_id("manuscript/메모"), stable_id("manuscript/메모"));
        assert_ne!(stable_id("manuscript/메모"), stable_id("planning/메모"));
        assert_eq!(stable_id("x").len(), 12);
        assert_eq!(rev_of(b"abc").len(), 16);
        assert_eq!(rev_of(b"abc"), "ba7816bf8f01cfea");
    }

    #[test]
    fn stamp_round_trips() {
        let t = parse_iso("2026-09-27T10:15:00.123Z").unwrap();
        let s = stamp(t);
        assert_eq!(s, "20260927-101500-123");
        assert_eq!(parse_stamp(&s), Some(t));
    }

    #[test]
    fn file_names_are_sanitized() {
        assert_eq!(safe_file_name("달빛 서점: 1부?", "x"), "달빛 서점_ 1부_");
        assert_eq!(safe_file_name("  ...  ", "새 작품"), "새 작품");
        assert_eq!(safe_file_name("con", "x"), "con_");
        assert_eq!(safe_file_name("COM1.txt", "x"), "COM1.txt_");
    }
}
