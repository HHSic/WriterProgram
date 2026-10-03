//! Chapters that are there but cannot be read (docs/safety-design.md S4), and
//! 고쳐 열기: mending them.
//!
//! A file that is missing may still be on its way from another device and is
//! left alone. A file that is there but empty, saved in another encoding
//! (Notepad's ANSI is EUC-KR) or with a broken front matter is reported
//! (`Overview::unreadable`) instead of silently dropping out of the tree.
//! Mending reads it with the import's encoding detection, keeps the original
//! next to it as `<id>.md.broken-<stamp>` and writes it again as UTF-8, with
//! a new front matter when the old one is broken.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;
use serde::Serialize;

use crate::count::count_blocks;
use crate::doc::{self, DocFile, Section, is_front_line, parse_doc, split_front_matter};
use crate::import;
use crate::markup::plain_text;
use crate::store::{modified_iso, now_iso, stamp};
use crate::{Error, Result};

/// How much of the text the preview shows.
const PREVIEW_CHARS: usize = 1500;

/// What reading a listed document's file found.
pub(crate) enum Reading {
    /// No file (yet): it may still be coming from another device.
    Missing,
    Read(Box<DocFile>),
    Unreadable(Trouble),
}

/// Why a file that is there cannot be read, for the screen.
#[derive(Debug, Clone)]
pub(crate) struct Trouble {
    pub title_guess: String,
    pub reason: String,
    /// 고쳐 열기 can mend it. Not for a file another program holds or that
    /// cannot be opened at all: trying again later is all there is.
    pub repairable: bool,
}

pub(crate) fn read(path: &Path) -> Reading {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Reading::Missing,
        Err(_) => {
            return Reading::Unreadable(Trouble {
                title_guess: String::new(),
                reason: "다른 프로그램이 쓰고 있거나 읽을 권한이 없는 파일".into(),
                repairable: false,
            });
        }
    };
    let id = stem(path);
    match doc::check_bytes(&bytes) {
        Ok(text) => Reading::Read(Box::new(parse_doc(&text, &id))),
        Err(damage) => Reading::Unreadable(Trouble {
            title_guess: salvage(&decode(&bytes).0, &id).meta.title,
            reason: damage.message().into(),
            repairable: true,
        }),
    }
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn decode(bytes: &[u8]) -> (String, &'static str) {
    import::decode(bytes, None)
}

/// The document as well as it can be read from `text`: its own front matter
/// when that is whole; what is left of a broken one (the `key: value` lines
/// right after `---`) and the rest as the body; or all of it as the body.
fn salvage(text: &str, id: &str) -> DocFile {
    let mut file = match text.strip_prefix("---\n") {
        Some(rest) if split_front_matter(text).0.is_none() => {
            let mut front = String::new();
            let mut offset = 0;
            for line in rest.split_inclusive('\n') {
                if !is_front_line(line) {
                    break;
                }
                front.push_str(line.trim_end_matches('\n'));
                front.push('\n');
                offset += line.len();
            }
            let body = rest[offset..].trim_start_matches('\n');
            parse_doc(&format!("---\n{front}---\n\n{body}"), id)
        }
        _ => parse_doc(text, id),
    };
    file.meta.id = id.to_string();
    file
}

/// Where the document's file is: a manuscript or planning file that is there.
fn file_of(root: &Path, id: &str) -> Result<(Section, PathBuf)> {
    doc::locate(root, id)
}

/// What 고쳐 열기 would make of a document, shown before the writer agrees.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MendPreview {
    pub id: String,
    pub section: Section,
    pub title: String,
    /// Why it could not be read.
    pub reason: String,
    /// How the text was read: `utf-8`, `utf-16` or `euc-kr`.
    pub encoding: String,
    /// The front matter is made anew (it was broken or missing).
    pub new_front: bool,
    /// The start of the text as it will read.
    pub text: String,
    /// Length of the whole text, spaces included.
    pub chars: u32,
}

/// Reads a document that cannot be read as it is, the way 고쳐 열기 would.
pub fn preview(root: &Path, id: &str) -> Result<MendPreview> {
    let (section, path) = file_of(root, id)?;
    let bytes = fs::read(&path).map_err(|e| Error::io(&path, e))?;
    let damage = match doc::check_bytes(&bytes) {
        Ok(_) => return Err(Error::Invalid("이미 열 수 있는 회차입니다".into())),
        Err(damage) => damage,
    };
    let (text, encoding) = decode(&bytes);
    let file = mended(&text, id, &path);
    let plain = plain_text(&file.body);
    Ok(MendPreview {
        id: id.to_string(),
        section,
        title: file.meta.title.clone(),
        reason: damage.message().into(),
        encoding: encoding.into(),
        new_front: split_front_matter(&text).0.is_none(),
        text: plain.chars().take(PREVIEW_CHARS).collect(),
        chars: count_blocks(&file.body).with_spaces,
    })
}

/// The document 고쳐 열기 writes: what `salvage` reads, with a creation time
/// (the file's own time when the front matter had none).
fn mended(text: &str, id: &str, path: &Path) -> DocFile {
    let mut file = salvage(text, id);
    if file.meta.created.is_empty() {
        file.meta.created = modified_iso(path).unwrap_or_else(now_iso);
    }
    if file.meta.status.is_empty() {
        file.meta.status = "draft".into();
    }
    file
}

/// What 고쳐 열기 did.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Mended {
    pub id: String,
    /// The original file, kept in the same folder, e.g. `abc.md.broken-20261003-101500-000`.
    pub kept: String,
}

/// Mends a document that cannot be read: keeps the original next to it as
/// `<id>.md.broken-<stamp>` (never removed) and writes it again as UTF-8.
pub fn mend(root: &Path, id: &str) -> Result<Mended> {
    let (_, path) = file_of(root, id)?;
    let bytes = fs::read(&path).map_err(|e| Error::io(&path, e))?;
    if doc::check_bytes(&bytes).is_ok() {
        return Err(Error::Invalid("이미 열 수 있는 회차입니다".into()));
    }
    let (text, _) = decode(&bytes);
    let file = mended(&text, id, &path);
    let kept_name = format!("{}.broken-{}", doc::file_name(id), stamp(Utc::now()));
    let kept = path.with_file_name(&kept_name);
    fs::copy(&path, &kept).map_err(|e| Error::io(&kept, e))?;
    doc::write_doc_file(&path, &file)?;
    Ok(Mended {
        id: id.to_string(),
        kept: kept_name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{self, NewProject, ProjectKind};
    use encoding_rs::EUC_KR;

    fn project() -> (tempfile::TempDir, PathBuf, String) {
        let dir = tempfile::tempdir().unwrap();
        let root = project::create(&NewProject {
            parent: dir.path().to_string_lossy().into_owned(),
            title: "고쳐 열기".into(),
            kind: ProjectKind::Webnovel,
            per_doc_goal: None,
            count_spaces: true,
            first_chapter: true,
            platform: None,
        })
        .unwrap();
        let id = project::load(&root).unwrap().parts[0].docs[0].clone();
        (dir, root, id)
    }

    fn chapter_path(root: &Path, id: &str) -> PathBuf {
        root.join("manuscript").join(doc::file_name(id))
    }

    fn kept_files(root: &Path, id: &str) -> Vec<PathBuf> {
        fs::read_dir(root.join("manuscript"))
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(&format!("{id}.md.broken-"))
            })
            .collect()
    }

    /// Writes `bytes` as the chapter, checks it shows as unreadable, mends it
    /// and checks it reads normally afterwards with the original kept.
    fn round_trip(make: impl Fn(&str) -> Vec<u8>, title: &str, body: &str) {
        let (_dir, root, id) = project();
        let path = chapter_path(&root, &id);
        let bytes = make(&id);
        fs::write(&path, &bytes).unwrap();

        let ov = project::open(&root).unwrap();
        assert!(ov.parts[0].docs.is_empty(), "left out of the readable list");
        assert_eq!(ov.unreadable.len(), 1);
        let u = &ov.unreadable[0];
        assert_eq!(u.id, id);
        assert_eq!(u.section, Section::Manuscript);
        assert_eq!(
            u.part.as_deref(),
            Some(project::load(&root).unwrap().parts[0].id.as_str())
        );
        assert!(u.repairable);
        assert_eq!(u.title_guess, title);
        assert!(doc::load(&root, &id).is_err());

        let p = preview(&root, &id).unwrap();
        assert_eq!(p.title, title);
        assert!(p.text.contains(body), "{:?}", p.text);

        let done = mend(&root, &id).unwrap();
        let kept = kept_files(&root, &id);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].file_name().unwrap().to_string_lossy(), done.kept);
        assert_eq!(
            fs::read(&kept[0]).unwrap(),
            bytes,
            "original kept as it was"
        );

        let ov = project::open(&root).unwrap();
        assert!(ov.unreadable.is_empty());
        assert_eq!(ov.parts[0].docs.len(), 1);
        assert_eq!(ov.parts[0].docs[0].title, title);
        let file = doc::load(&root, &id).unwrap();
        assert_eq!(file.meta.id, id);
        assert!(plain_text(&file.body).contains(body));
        assert!(!file.meta.created.is_empty());
        assert!(mend(&root, &id).is_err(), "nothing left to mend");
    }

    #[test]
    fn euc_kr_file_is_mended() {
        let make = |id: &str| {
            let src = format!(
                "---\nid: \"{id}\"\ntitle: \"비에 젖은 손님\"\nstatus: \"draft\"\n---\n\n폐점 직전 손님이 왔다.\n"
            );
            EUC_KR.encode(&src).0.into_owned()
        };
        round_trip(make, "비에 젖은 손님", "폐점 직전 손님이 왔다.");
    }

    #[test]
    fn broken_front_matter_keeps_the_body() {
        let make = |id: &str| {
            format!("---\nid: \"{id}\"\ntitle: \"첫 손님\"\n폐점 직전 손님이 왔다.\n\n둘째 문단.\n")
                .into_bytes()
        };
        round_trip(make, "첫 손님", "둘째 문단.");
    }

    #[test]
    fn empty_file_is_mended() {
        round_trip(|_| Vec::new(), "", "");
    }

    #[test]
    fn missing_files_are_not_unreadable() {
        let (_dir, root, id) = project();
        fs::remove_file(chapter_path(&root, &id)).unwrap();
        let ov = project::overview(&root).unwrap();
        assert!(ov.unreadable.is_empty());
        assert!(ov.parts[0].docs.is_empty());
    }

    #[test]
    fn hand_written_files_without_front_matter_still_read() {
        assert!(doc::check_bytes("---\n장면이 바뀐다.\n".as_bytes()).is_ok());
        assert!(doc::check_bytes("그냥 글.\n".as_bytes()).is_ok());
        assert_eq!(doc::check_bytes(b"\n\n"), Err(doc::Damage::Empty));
        assert_eq!(
            doc::check_bytes("---\ntitle: \"a\"\n본문".as_bytes()),
            Err(doc::Damage::FrontMatter)
        );
    }
}
