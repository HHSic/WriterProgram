//! Moving a project folder somewhere else, for example into a folder a sync
//! program keeps in step with other devices.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use super::{load, unique_dir};
use crate::store::{new_id, rename_retry};
use crate::{Error, Result};

/// Where a project went, from `relocate`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Moved {
    pub root: String,
    /// The old folder could not be removed completely (a program was holding
    /// a file); what is left of it stays where it was.
    pub left_behind: bool,
}

/// Moves the project folder into `dest_parent`, for example into a folder
/// that OneDrive keeps in step with other devices. On the same disk this is
/// one rename. Otherwise the folder is copied, every file is checked against
/// the original, and only then is the old folder removed.
pub fn relocate(root: &Path, dest_parent: &Path) -> Result<Moved> {
    load(root)?;
    let name = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| Error::Invalid("작품 폴더가 올바르지 않음".into()))?;
    fs::create_dir_all(dest_parent).map_err(|e| Error::io(dest_parent, e))?;
    let (from, to) = (canonical(root)?, canonical(dest_parent)?);
    if to.starts_with(&from) {
        return Err(Error::Invalid("작품 폴더 안으로는 옮길 수 없음".into()));
    }
    if from.parent() == Some(to.as_path()) {
        return Err(Error::Invalid("이미 그 위치에 있음".into()));
    }
    let target = unique_dir(dest_parent, &name);

    if rename_dir(root, &target).is_ok() {
        return Ok(Moved {
            root: target.to_string_lossy().into_owned(),
            left_behind: false,
        });
    }

    let staging = dest_parent.join(format!(".{name}.{}.moving", new_id()));
    let copied = copy_tree(root, &staging).and_then(|()| same_tree(root, &staging));
    if let Err(e) = copied {
        let _ = fs::remove_dir_all(&staging);
        return Err(e);
    }
    if let Err(e) = rename_dir(&staging, &target) {
        let _ = fs::remove_dir_all(&staging);
        return Err(Error::io(&target, e));
    }
    Ok(Moved {
        root: target.to_string_lossy().into_owned(),
        left_behind: fs::remove_dir_all(root).is_err(),
    })
}

fn canonical(path: &Path) -> Result<PathBuf> {
    fs::canonicalize(path).map_err(|e| Error::io(path, e))
}

/// Renames a folder, retrying briefly while a sync program or virus scanner
/// holds a file in it.
fn rename_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    rename_retry(from, to, 4, Duration::from_millis(80))
}

fn copy_tree(from: &Path, to: &Path) -> Result<()> {
    fs::create_dir_all(to).map_err(|e| Error::io(to, e))?;
    for entry in fs::read_dir(from).map_err(|e| Error::io(from, e))? {
        let entry = entry.map_err(|e| Error::io(from, e))?;
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        if entry
            .file_type()
            .map_err(|e| Error::io(&source, e))?
            .is_dir()
        {
            copy_tree(&source, &target)?;
        } else {
            fs::copy(&source, &target).map_err(|e| Error::io(&source, e))?;
        }
    }
    Ok(())
}

/// Checks that `copy` holds exactly the files of `original`, byte for byte.
fn same_tree(original: &Path, copy: &Path) -> Result<()> {
    for entry in fs::read_dir(original).map_err(|e| Error::io(original, e))? {
        let entry = entry.map_err(|e| Error::io(original, e))?;
        let (a, b) = (entry.path(), copy.join(entry.file_name()));
        if entry.file_type().map_err(|e| Error::io(&a, e))?.is_dir() {
            same_tree(&a, &b)?;
            continue;
        }
        let same = matches!(
            (fs::read(&a), fs::read(&b)),
            (Ok(x), Ok(y)) if x == y
        );
        if !same {
            return Err(Error::Invalid(
                "옮긴 파일이 원래 파일과 달라 옮기지 않음. 원래 폴더는 그대로 있습니다.".into(),
            ));
        }
    }
    Ok(())
}
