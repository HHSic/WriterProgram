//! What the sync engine needs from a drive, for one project folder on it.

use crate::Result;

/// A file in the project folder on the drive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteFile {
    /// Path inside the project folder, with `/` between parts.
    pub path: String,
    /// The drive's version of the file; it changes whenever the file does.
    pub rev: String,
}

/// How a conditional write went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Put {
    /// Written; the file's new version.
    Done(String),
    /// Someone changed (or made, or removed) the file first. Nothing written.
    Changed,
}

pub trait Remote {
    /// Every file in the project folder.
    fn list(&mut self) -> Result<Vec<RemoteFile>>;

    fn get(&mut self, path: &str) -> Result<Vec<u8>>;

    /// Writes a file if it is still at version `expected`, or does not exist
    /// yet when `expected` is `None`.
    fn put(&mut self, path: &str, bytes: &[u8], expected: Option<&str>) -> Result<Put>;

    /// Removes a file if it is still at version `expected`. False when it
    /// changed first (then it stays).
    fn remove(&mut self, path: &str, expected: &str) -> Result<bool>;
}

/// Splits `a/b/c.md` into its folders and name.
pub(crate) fn split(path: &str) -> (Vec<&str>, &str) {
    let mut parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let name = parts.pop().unwrap_or("");
    (parts, name)
}

/// Checks a path from a drive before it becomes a file name here.
pub(crate) fn safe_path(path: &str) -> bool {
    !path.is_empty()
        && !path.starts_with('/')
        && !path.contains('\\')
        && !path.contains(':')
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
}
