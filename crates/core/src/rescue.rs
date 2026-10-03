//! Rescue copies (비상 보관): when saving into the project folder keeps
//! failing (the drive went away, the disk is full, the folder became read
//! only), the screen writes what it could not save into the app's own data
//! folder instead, so the writing survives a crash or a closed window.
//!
//! Layout: `<base>/<project id>/<item>-<YYYYMMDD-HHMMSS>.md`, where `<item>` is
//! a document id for a chapter's text, or `meta-<id>`, `card-<id>`,
//! `note-<id>` for the other things the screen saves. A chapter's file holds
//! its text in the manuscript markup (`markup::write_body`), so it can be read
//! back and compared with the chapter. Files the writer has dealt with move to
//! `<base>/<project id>/old/`; nothing here is ever deleted.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local};
use serde::Serialize;

use crate::markup::{Block, parse_body};
use crate::store::{atomic_write, modified_iso, read_text};
use crate::{Error, Result};

/// Where files the writer has dealt with go, inside a project's folder.
const OLD: &str = "old";
/// Length of the time stamp at the end of a file name (`20261003-142530`).
const STAMP_LEN: usize = 15;

/// One rescue copy.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RescueFile {
    /// The document id, or `meta-<id>`, `card-<id>`, `note-<id>`.
    pub item: String,
    pub path: String,
    /// When the file was last written (RFC 3339, UTC).
    pub saved: String,
}

/// A name part that is safe in a file name on every system.
fn check_name(name: &str) -> Result<()> {
    let ok = !name.is_empty()
        && name.len() <= 80
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "비상 보관 이름이 올바르지 않음: {name}"
        )))
    }
}

/// The time stamp in a rescue file's name, in local time.
pub fn stamp(t: DateTime<Local>) -> String {
    t.format("%Y%m%d-%H%M%S").to_string()
}

fn check_stamp(stamp: &str) -> Result<()> {
    let ok = stamp.len() == STAMP_LEN
        && stamp
            .char_indices()
            .all(|(i, c)| if i == 8 { c == '-' } else { c.is_ascii_digit() });
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(format!(
            "비상 보관 시각이 올바르지 않음: {stamp}"
        )))
    }
}

/// The folder of one project's rescue copies.
pub fn folder(base: &Path, project_id: &str) -> Result<PathBuf> {
    check_name(project_id)?;
    Ok(base.join(project_id))
}

/// Writes `text` as the rescue copy of `item`. The same item and stamp (one
/// run of failed saves) write the same file again, so retries keep one file
/// with the latest text instead of a new one every minute.
pub fn write(
    base: &Path,
    project_id: &str,
    item: &str,
    stamp: &str,
    text: &str,
) -> Result<PathBuf> {
    check_name(item)?;
    check_stamp(stamp)?;
    let path = folder(base, project_id)?.join(format!("{item}-{stamp}.md"));
    atomic_write(&path, text.as_bytes())?;
    Ok(path)
}

/// The item a rescue file belongs to, from its name.
fn item_of(name: &str) -> Option<&str> {
    let stem = name.strip_suffix(".md")?;
    let cut = stem.len().checked_sub(STAMP_LEN + 1)?;
    let (item, rest) = stem.split_at(cut);
    let stamp = rest.strip_prefix('-')?;
    (check_name(item).is_ok() && check_stamp(stamp).is_ok()).then_some(item)
}

/// The project's rescue copies not dealt with yet, newest first.
pub fn list(base: &Path, project_id: &str) -> Result<Vec<RescueFile>> {
    let dir = folder(base, project_id)?;
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(&dir, e)),
    };
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(item) = item_of(&name) else { continue };
        files.push(RescueFile {
            item: item.to_string(),
            path: path.to_string_lossy().into_owned(),
            saved: modified_iso(&path).unwrap_or_default(),
        });
    }
    files.sort_by(|a, b| b.saved.cmp(&a.saved).then_with(|| a.path.cmp(&b.path)));
    Ok(files)
}

/// A path given by the screen, checked to be a rescue copy under `base`.
fn inside(base: &Path, path: &str) -> Result<PathBuf> {
    let path = PathBuf::from(path);
    let not_ours = || Error::Invalid("비상 보관 파일이 아님".into());
    let canon = path.canonicalize().map_err(|e| Error::io(&path, e))?;
    let base = base.canonicalize().map_err(|e| Error::io(base, e))?;
    let name = canon
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(not_ours)?;
    if !canon.starts_with(&base) || item_of(name).is_none() {
        return Err(not_ours());
    }
    Ok(canon)
}

/// The text of a chapter's rescue copy.
pub fn load_body(base: &Path, path: &str) -> Result<Vec<Block>> {
    let path = inside(base, path)?;
    Ok(parse_body(&read_text(&path)?))
}

/// Moves a rescue copy the writer has dealt with into `old/`, out of the
/// list. It is kept, not deleted.
pub fn set_aside(base: &Path, path: &str) -> Result<()> {
    let path = inside(base, path)?;
    let dir = path
        .parent()
        .ok_or_else(|| Error::Invalid("비상 보관 파일이 아님".into()))?;
    let old = dir.join(OLD);
    fs::create_dir_all(&old).map_err(|e| Error::io(&old, e))?;
    let name = path.file_name().unwrap_or_default();
    let to = old.join(name);
    fs::rename(&path, &to).map_err(|e| Error::io(&path, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup::{Block, write_body};
    use chrono::TimeZone;

    #[test]
    fn writes_lists_loads_and_sets_aside() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();
        let at = Local.with_ymd_and_hms(2026, 10, 3, 14, 25, 30).unwrap();
        assert_eq!(stamp(at), "20261003-142530");

        let text = write_body(&[Block::text("첫 문단"), Block::text("둘째 문단")]);
        let path = write(base, "proj1", "abcdefghijkl", &stamp(at), &text).unwrap();
        assert!(path.ends_with("proj1/abcdefghijkl-20261003-142530.md"));
        write(base, "proj1", "card-xyz", &stamp(at), "카드").unwrap();

        let files = list(base, "proj1").unwrap();
        assert_eq!(files.len(), 2);
        let doc = files.iter().find(|f| f.item == "abcdefghijkl").unwrap();
        assert!(!doc.saved.is_empty());
        assert_eq!(
            load_body(base, &doc.path).unwrap(),
            vec![Block::text("첫 문단"), Block::text("둘째 문단")]
        );

        // Writing the same run again keeps one file with the latest text.
        write(base, "proj1", "abcdefghijkl", &stamp(at), "새 글").unwrap();
        assert_eq!(list(base, "proj1").unwrap().len(), 2);

        set_aside(base, &doc.path).unwrap();
        let left = list(base, "proj1").unwrap();
        assert_eq!(left.len(), 1);
        assert_eq!(left[0].item, "card-xyz");
        assert!(
            base.join("proj1")
                .join(OLD)
                .join("abcdefghijkl-20261003-142530.md")
                .is_file()
        );
    }

    #[test]
    fn no_folder_is_an_empty_list() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(list(tmp.path(), "nothing").unwrap().is_empty());
    }

    #[test]
    fn refuses_unsafe_names_and_foreign_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path().join("rescue");
        assert!(write(&base, "../x", "doc", "20261003-142530", "").is_err());
        assert!(write(&base, "p", "a/b", "20261003-142530", "").is_err());
        assert!(write(&base, "p", "doc", "2026-10-03", "").is_err());

        fs::create_dir_all(&base).unwrap();
        let outside = tmp.path().join("doc-20261003-142530.md");
        fs::write(&outside, "글").unwrap();
        assert!(load_body(&base, &outside.to_string_lossy()).is_err());
        assert!(set_aside(&base, &outside.to_string_lossy()).is_err());
    }

    #[test]
    fn reads_the_item_from_a_name() {
        assert_eq!(item_of("abc-20261003-142530.md"), Some("abc"));
        assert_eq!(item_of("meta-abc-20261003-142530.md"), Some("meta-abc"));
        assert_eq!(item_of("abc.md"), None);
        assert_eq!(item_of("abc-20261003-142530.txt"), None);
    }
}
