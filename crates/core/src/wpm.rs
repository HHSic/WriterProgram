//! Portable WriterProgram manuscript (.wpm): a versioned ZIP of a project.
//! Work in the normal folder format, then replace the package atomically.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::project;
use crate::store::{new_id, rename_retry};
use crate::{Error, Result};

const MANIFEST: &str = "wpm.json";
const KIND: &str = "writerprogram-manuscript";
const MAX_FILES: usize = 100_000;
const MAX_BYTES: u64 = 1024 * 1024 * 1024;
const DIRS: &[&str] = &[
    "manuscript",
    "planning",
    "cards",
    "notes",
    ".snapshots",
    ".trash",
    ".journal",
    ".exchanges",
    ".backup",
];

#[derive(Serialize, Deserialize)]
struct Manifest {
    kind: String,
    format: u32,
}

fn invalid(message: &str) -> Error {
    Error::Invalid(format!("작품 파일(.wpm): {message}"))
}

/// A streaming fingerprint, used to avoid overwriting another device's package.
pub fn fingerprint(path: &Path) -> Result<String> {
    let mut file = File::open(path).map_err(|e| Error::io(path, e))?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| Error::io(path, e))?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

/// Canonical, portable paths only; no absolute paths, traversal or Windows aliases.
fn checked_name(name: &str, directory: bool) -> Result<String> {
    let name = if directory {
        name.strip_suffix('/').unwrap_or(name)
    } else {
        name
    };
    let parts: Vec<_> = name.split('/').collect();
    if parts.len() > 32
        || parts.iter().any(|p| {
            p.is_empty()
                || *p == "."
                || *p == ".."
                || p.ends_with(['.', ' '])
                || p.chars()
                    .any(|c| c.is_control() || "\\:<>\"|?*".contains(c))
                || {
                    let stem = p.split('.').next().unwrap_or("").to_ascii_uppercase();
                    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
                        || ["COM", "LPT"].iter().any(|prefix| {
                            stem.strip_prefix(prefix).is_some_and(|n| {
                                matches!(n, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
                            })
                        })
                }
        })
    {
        return Err(invalid("올바르지 않은 파일 경로가 있음"));
    }
    let allowed = if parts.len() == 1 && !directory {
        matches!(parts[0], MANIFEST | project::PROJECT_FILE)
    } else {
        DIRS.contains(&parts[0])
    };
    if !allowed {
        return Err(invalid("작품에 속하지 않는 파일이 있음"));
    }
    Ok(name.to_owned())
}

fn collect(root: &Path, dir: &Path, files: &mut Vec<(String, PathBuf)>) -> Result<()> {
    for item in fs::read_dir(dir).map_err(|e| Error::io(dir, e))? {
        let item = item.map_err(|e| Error::io(dir, e))?;
        let path = item.path();
        let kind = item.file_type().map_err(|e| Error::io(&path, e))?;
        let name = path
            .strip_prefix(root)
            .expect("inside root")
            .components()
            .map(|c| {
                c.as_os_str()
                    .to_str()
                    .ok_or_else(|| invalid("파일 이름을 읽을 수 없음"))
            })
            .collect::<Result<Vec<_>>>()?
            .join("/");
        if name.ends_with(".tmp") {
            continue;
        }
        checked_name(&name, kind.is_dir())?;
        if kind.is_dir() {
            collect(root, &path, files)?;
        } else if kind.is_file() {
            files.push((name, path));
        } else {
            return Err(invalid("심볼릭 링크나 특수 파일은 담을 수 없음"));
        }
        if files.len() > MAX_FILES {
            return Err(invalid("파일 수가 너무 많음"));
        }
    }
    Ok(())
}

/// Saves all project data, including records, trash and creation proofs.
pub fn save(root: &Path, dest: &Path) -> Result<()> {
    project::load(root)?;
    if !dest
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("wpm"))
    {
        return Err(invalid("파일 이름은 .wpm으로 끝나야 함"));
    }
    let parent = dest
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    let canonical_root = fs::canonicalize(root).map_err(|e| Error::io(root, e))?;
    if fs::canonicalize(parent)
        .map_err(|e| Error::io(parent, e))?
        .starts_with(&canonical_root)
    {
        return Err(invalid("작품 폴더 밖에 저장 위치를 골라 주세요"));
    }
    let project_file = root.join(project::PROJECT_FILE);
    if fs::symlink_metadata(&project_file)
        .map_err(|e| Error::io(&project_file, e))?
        .file_type()
        .is_symlink()
    {
        return Err(invalid("심볼릭 링크는 담을 수 없음"));
    }
    let mut files = vec![(project::PROJECT_FILE.to_owned(), project_file)];
    for name in DIRS {
        let path = root.join(name);
        match fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_dir() => collect(root, &path, &mut files)?,
            Ok(_) => return Err(invalid("작품 폴더에 심볼릭 링크나 특수 파일이 있음")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(Error::io(&path, e)),
        }
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let tmp = parent.join(format!(".wpm-{}.tmp", new_id()));
    let result = (|| {
        let file = File::create_new(&tmp).map_err(|e| Error::io(&tmp, e))?;
        let mut zip = ZipWriter::new(file);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        let zip_error = |e| Error::Invalid(format!("작품 파일을 만들지 못함 ({e})"));
        zip.start_file(MANIFEST, opts).map_err(zip_error)?;
        zip.write_all(
            &serde_json::to_vec(&Manifest {
                kind: KIND.into(),
                format: 1,
            })
            .expect("manifest serializes"),
        )
        .map_err(|e| Error::io(&tmp, e))?;
        let mut total = 0u64;
        let mut names = HashSet::new();
        for (name, path) in &files {
            if !names.insert(name.to_lowercase()) {
                return Err(invalid("대소문자만 다른 파일 이름이 있음"));
            }
            let mut source = File::open(path).map_err(|e| Error::io(path, e))?;
            zip.start_file(name, opts).map_err(zip_error)?;
            let copied = std::io::copy(
                &mut Read::by_ref(&mut source).take(MAX_BYTES - total + 1),
                &mut zip,
            )
            .map_err(|e| Error::io(path, e))?;
            total += copied;
            if total > MAX_BYTES {
                return Err(invalid("작품 파일은 풀었을 때 1GB를 넘을 수 없음"));
            }
        }
        let file = zip.finish().map_err(zip_error)?;
        file.sync_all().map_err(|e| Error::io(&tmp, e))?;
        drop(file);
        rename_retry(&tmp, dest, 6, Duration::from_millis(40)).map_err(|e| Error::io(dest, e))?;
        #[cfg(unix)]
        if let Ok(dir) = File::open(parent) {
            let _ = dir.sync_all();
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Opens into a newly created working folder. Existing folders are never overwritten.
/// A partial working folder is removed on failure; the source package is untouched.
pub fn unpack(source: &Path, parent: &Path) -> Result<PathBuf> {
    let file = File::open(source).map_err(|e| Error::io(source, e))?;
    let mut zip = ZipArchive::new(file).map_err(|_| invalid("손상되었거나 ZIP 형식이 아님"))?;
    if zip.len() > MAX_FILES + 1 {
        return Err(invalid("파일 수가 너무 많음"));
    }
    let mut names = HashSet::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        let entry = zip
            .by_index(i)
            .map_err(|_| invalid("파일 목록을 읽을 수 없음"))?;
        let name = checked_name(entry.name(), entry.is_dir())?;
        if !names.insert(name.to_lowercase()) {
            return Err(invalid("겹치는 파일 경로가 있음"));
        }
        if entry
            .unix_mode()
            .is_some_and(|mode| !matches!(mode & 0o170000, 0 | 0o100000 | 0o040000))
        {
            return Err(invalid("심볼릭 링크나 특수 파일은 열 수 없음"));
        }
        total = total
            .checked_add(entry.size())
            .ok_or_else(|| invalid("파일 크기가 너무 큼"))?;
        if total > MAX_BYTES {
            return Err(invalid("작품 파일은 풀었을 때 1GB를 넘을 수 없음"));
        }
    }
    let manifest: Manifest = {
        let entry = zip
            .by_name(MANIFEST)
            .map_err(|_| invalid("WriterProgram 작품 파일이 아님"))?;
        if entry.size() > 4096 {
            return Err(invalid("파일 형식 정보가 너무 큼"));
        }
        serde_json::from_reader(entry.take(4096))
            .map_err(|_| invalid("파일 형식 정보를 읽을 수 없음"))?
    };
    if manifest.kind != KIND {
        return Err(invalid("WriterProgram 작품 파일이 아님"));
    }
    if manifest.format != 1 {
        return Err(invalid(
            "지원하지 않는 작품 파일 버전입니다. 앱을 업데이트해 주세요.",
        ));
    }
    fs::create_dir_all(parent).map_err(|e| Error::io(parent, e))?;
    let root = parent.join(new_id());
    fs::create_dir(&root).map_err(|e| Error::io(&root, e))?;
    let result = (|| {
        let mut written = 0u64;
        for i in 0..zip.len() {
            let mut entry = zip
                .by_index(i)
                .map_err(|_| invalid("파일을 읽을 수 없음"))?;
            if entry.name() == MANIFEST {
                continue;
            }
            let path = root.join(checked_name(entry.name(), entry.is_dir())?);
            if entry.is_dir() {
                fs::create_dir_all(&path).map_err(|e| Error::io(&path, e))?;
                continue;
            }
            fs::create_dir_all(path.parent().expect("inside root"))
                .map_err(|e| Error::io(&path, e))?;
            let mut out = File::create_new(&path).map_err(|e| Error::io(&path, e))?;
            let expected = entry.size();
            let copied = std::io::copy(
                &mut Read::by_ref(&mut entry).take(MAX_BYTES - written + 1),
                &mut out,
            )
            .map_err(|e| Error::io(&path, e))?;
            written += copied;
            if copied != expected || written > MAX_BYTES {
                return Err(invalid("파일 크기가 잘못되었거나 너무 큼"));
            }
            out.sync_all().map_err(|e| Error::io(&path, e))?;
        }
        project::load(&root)?;
        Ok(root.clone())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&root);
    }
    result
}
