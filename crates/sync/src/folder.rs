//! A plain folder acting as a drive (흉내 드라이브): for tests, and for trying
//! the sync without an account. Versions are fingerprints of the content.

use std::fs;
use std::path::{Path, PathBuf};

use writer_core::store::rev_of;

use crate::remote::{Put, Remote, RemoteFile, safe_path};
use crate::{Error, Result};

pub struct FolderRemote {
    root: PathBuf,
}

impl FolderRemote {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        FolderRemote { root: root.into() }
    }

    fn path(&self, path: &str) -> Result<PathBuf> {
        if !safe_path(path) {
            return Err(Error::Invalid(format!("올바르지 않은 경로: {path}")));
        }
        Ok(self.root.join(path))
    }

    fn rev(&self, path: &Path) -> Option<String> {
        fs::read(path).ok().map(|bytes| rev_of(&bytes))
    }
}

fn walk(dir: &Path, prefix: &str, out: &mut Vec<RemoteFile>) -> Result<()> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(Error::io(dir, e)),
    };
    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = format!("{prefix}{name}");
        if entry.path().is_dir() {
            walk(&entry.path(), &format!("{path}/"), out)?;
        } else {
            let bytes = fs::read(entry.path()).map_err(|e| Error::io(&entry.path(), e))?;
            out.push(RemoteFile {
                path,
                rev: rev_of(&bytes),
            });
        }
    }
    Ok(())
}

impl Remote for FolderRemote {
    fn list(&mut self) -> Result<Vec<RemoteFile>> {
        let mut out = Vec::new();
        walk(&self.root, "", &mut out)?;
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    fn get(&mut self, path: &str) -> Result<Vec<u8>> {
        let full = self.path(path)?;
        fs::read(&full).map_err(|e| Error::io(&full, e))
    }

    fn put(&mut self, path: &str, bytes: &[u8], expected: Option<&str>) -> Result<Put> {
        let full = self.path(path)?;
        if self.rev(&full).as_deref() != expected {
            return Ok(Put::Changed);
        }
        if let Some(parent) = full.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
        }
        fs::write(&full, bytes).map_err(|e| Error::io(&full, e))?;
        Ok(Put::Done(rev_of(bytes)))
    }

    fn remove(&mut self, path: &str, expected: &str) -> Result<bool> {
        let full = self.path(path)?;
        match self.rev(&full) {
            Some(rev) if rev == expected => {
                fs::remove_file(&full).map_err(|e| Error::io(&full, e))?;
                Ok(true)
            }
            // Gone already: nothing to remove.
            None => Ok(true),
            Some(_) => Ok(false),
        }
    }
}
