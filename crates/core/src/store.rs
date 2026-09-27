//! File primitives: atomic writes, reading text, ids and timestamps.

use std::collections::hash_map::RandomState;
use std::fs::{self, File};
use std::hash::{BuildHasher, Hasher};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Datelike, SecondsFormat, Timelike, Utc};

use crate::{Error, Result};

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

    if let Err(e) = rename_with_retry(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(Error::io(path, e));
    }

    #[cfg(unix)]
    if let Ok(d) = File::open(dir) {
        let _ = d.sync_all();
    }
    Ok(())
}

/// Sync clients and virus scanners briefly lock files on Windows, which makes a
/// rename fail with "access denied". A few short retries ride that out.
fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut last = None;
    for attempt in 0..6 {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
                last = Some(e);
                thread::sleep(Duration::from_millis(40 * (attempt + 1)));
            }
            Err(e) => return Err(e),
        }
    }
    Err(last.expect("retried at least once"))
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
    let mut v = hasher.finish();
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

pub fn parse_iso(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

/// Compact UTC stamp for file names that sorts by time: `20260927-101500-123`.
pub fn stamp(t: DateTime<Utc>) -> String {
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}-{:03}",
        t.year(),
        t.month(),
        t.day(),
        t.hour(),
        t.minute(),
        t.second(),
        t.timestamp_subsec_millis()
    )
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
