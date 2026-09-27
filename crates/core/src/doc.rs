//! Document files: one manuscript chapter or planning document per `.md` file,
//! with a small front matter block for its title, synopsis and status.
//!
//! ```text
//! ---
//! id: "k7q2m9x4t1ab"
//! title: "비에 젖은 손님"
//! synopsis: "폐점 직전 찾아온 손님이 대여 카드를 내민다."
//! status: "draft"
//! target: 5000
//! created: "2026-09-27T01:00:00.000Z"
//! ---
//!
//! 본문 (markup.rs)
//! ```
//!
//! Front matter values are JSON scalars, which YAML also reads. Keys this
//! version does not know are kept as they are.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::count::{Counts, count_blocks};
use crate::markup::{Block, parse_body, write_body};
use crate::project::{MANUSCRIPT_DIR, PLANNING_DIR};
use crate::store::{atomic_write, now_iso, read_text};
use crate::{Error, Result, snapshot};

/// Status values. Common ones first, then web novel, then print.
pub const STATUSES: [&str; 10] = [
    "draft",
    "revise",
    "done",
    "stock",
    "scheduled",
    "published",
    "final",
    "proof1",
    "proof2",
    "proof3",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocMeta {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub synopsis: String,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default)]
    pub target: Option<u32>,
    #[serde(default)]
    pub created: String,
    /// Front matter lines this version does not know, as `(key, raw value)`.
    #[serde(skip)]
    pub extra: Vec<(String, String)>,
}

fn default_status() -> String {
    "draft".into()
}

impl DocMeta {
    pub fn new(id: &str, title: &str) -> Self {
        DocMeta {
            id: id.into(),
            title: title.into(),
            synopsis: String::new(),
            status: default_status(),
            target: None,
            created: now_iso(),
            extra: Vec::new(),
        }
    }

    pub fn extra_str(&self, key: &str) -> Option<String> {
        self.extra
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, raw)| decode_value(raw))
    }

    pub fn set_extra_str(&mut self, key: &str, value: &str) {
        let raw = serde_json::to_string(value).expect("strings serialize");
        match self.extra.iter_mut().find(|(k, _)| k == key) {
            Some(entry) => entry.1 = raw,
            None => self.extra.push((key.into(), raw)),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DocFile {
    pub meta: DocMeta,
    pub body: Vec<Block>,
}

fn decode_value(raw: &str) -> String {
    if raw.starts_with('"') {
        serde_json::from_str::<String>(raw).unwrap_or_else(|_| raw.trim_matches('"').to_string())
    } else {
        raw.to_string()
    }
}

fn encode(value: &str) -> String {
    serde_json::to_string(value).expect("strings serialize")
}

/// Reads a document file's text. A file without front matter (for example one
/// dropped into the folder by hand) gets its id from `fallback_id`.
pub fn parse_doc(src: &str, fallback_id: &str) -> DocFile {
    let (front, body) = split_front_matter(src);
    let mut meta = DocMeta::new(fallback_id, "");
    meta.created = String::new();
    if let Some(front) = front {
        for line in front.lines() {
            let Some((key, raw)) = line.split_once(':') else {
                continue;
            };
            let key = key.trim();
            let raw = raw.trim();
            match key {
                "id" => meta.id = decode_value(raw),
                "title" => meta.title = decode_value(raw),
                "synopsis" => meta.synopsis = decode_value(raw),
                "status" => meta.status = decode_value(raw),
                "target" => meta.target = decode_value(raw).parse().ok(),
                "created" => meta.created = decode_value(raw),
                _ => meta.extra.push((key.to_string(), raw.to_string())),
            }
        }
    }
    if meta.id.is_empty() {
        meta.id = fallback_id.to_string();
    }
    DocFile {
        meta,
        body: parse_body(body),
    }
}

fn split_front_matter(src: &str) -> (Option<&str>, &str) {
    let Some(rest) = src.strip_prefix("---\n") else {
        return (None, src);
    };
    let mut offset = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end_matches('\n') == "---" {
            let front = &rest[..offset];
            let body = &rest[offset + line.len()..];
            return (Some(front), body.trim_start_matches('\n'));
        }
        offset += line.len();
    }
    (None, src)
}

pub fn write_doc(doc: &DocFile) -> String {
    let m = &doc.meta;
    let mut out = String::from("---\n");
    out.push_str(&format!("id: {}\n", encode(&m.id)));
    out.push_str(&format!("title: {}\n", encode(&m.title)));
    out.push_str(&format!("synopsis: {}\n", encode(&m.synopsis)));
    out.push_str(&format!("status: {}\n", encode(&m.status)));
    if let Some(target) = m.target {
        out.push_str(&format!("target: {target}\n"));
    }
    out.push_str(&format!("created: {}\n", encode(&m.created)));
    for (key, raw) in &m.extra {
        out.push_str(&format!("{key}: {raw}\n"));
    }
    out.push_str("---\n\n");
    out.push_str(&write_body(&doc.body));
    out
}

/// Which folder a document lives in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Section {
    Manuscript,
    Planning,
}

impl Section {
    pub fn dir(self) -> &'static str {
        match self {
            Section::Manuscript => MANUSCRIPT_DIR,
            Section::Planning => PLANNING_DIR,
        }
    }
}

pub fn file_name(id: &str) -> String {
    format!("{id}.md")
}

/// Finds a document file by id in either folder.
pub fn locate(root: &Path, id: &str) -> Result<(Section, PathBuf)> {
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(Error::Invalid("올바르지 않은 문서 이름".into()));
    }
    for section in [Section::Manuscript, Section::Planning] {
        let path = root.join(section.dir()).join(file_name(id));
        if path.is_file() {
            return Ok((section, path));
        }
    }
    Err(Error::NotFound("문서를 찾을 수 없음".into()))
}

pub fn read_doc(path: &Path) -> Result<DocFile> {
    let fallback = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(parse_doc(&read_text(path)?, &fallback))
}

pub fn write_doc_file(path: &Path, doc: &DocFile) -> Result<()> {
    atomic_write(path, write_doc(doc).as_bytes())
}

pub fn load(root: &Path, id: &str) -> Result<DocFile> {
    let (_, path) = locate(root, id)?;
    read_doc(&path)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveOutcome {
    pub counts: Counts,
    /// Set when this save also kept an automatic record of the previous text.
    pub snapshot: Option<snapshot::SnapshotInfo>,
}

/// Replaces a document's body, keeping its front matter as it is on disk, so
/// that title or status changes made elsewhere are never overwritten by the
/// editor. Keeps an automatic record first when one is due.
pub fn save_body(
    root: &Path,
    id: &str,
    body: Vec<Block>,
    auto_record_every: chrono::Duration,
) -> Result<SaveOutcome> {
    let (_, path) = locate(root, id)?;
    let current = read_doc(&path)?;
    let counts = count_blocks(&body);
    if current.body == body {
        return Ok(SaveOutcome {
            counts,
            snapshot: None,
        });
    }
    let snapshot = snapshot::auto_if_due(root, &current, auto_record_every)?;
    write_doc_file(
        &path,
        &DocFile {
            meta: current.meta,
            body,
        },
    )?;
    Ok(SaveOutcome { counts, snapshot })
}

/// Fields that can be changed without touching the body. `None` leaves a field
/// as it is; `target: Some(None)` clears the target.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MetaPatch {
    pub title: Option<String>,
    pub synopsis: Option<String>,
    pub status: Option<String>,
    #[serde(default, with = "double_option")]
    pub target: Option<Option<u32>>,
}

mod double_option {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        Option::<T>::deserialize(d).map(Some)
    }
}

pub fn update_meta(root: &Path, id: &str, patch: &MetaPatch) -> Result<DocMeta> {
    let (_, path) = locate(root, id)?;
    let mut doc = read_doc(&path)?;
    if let Some(title) = &patch.title {
        doc.meta.title = title.trim().chars().take(200).collect();
    }
    if let Some(synopsis) = &patch.synopsis {
        doc.meta.synopsis = synopsis.trim().to_string();
    }
    if let Some(status) = &patch.status {
        if !STATUSES.contains(&status.as_str()) {
            return Err(Error::Invalid(format!("알 수 없는 상태: {status}")));
        }
        doc.meta.status = status.clone();
    }
    if let Some(target) = patch.target {
        doc.meta.target = target.filter(|t| *t > 0);
    }
    write_doc_file(&path, &doc)?;
    Ok(doc.meta)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_matter_round_trips() {
        let mut meta = DocMeta::new("abc", "비에 젖은 손님: \"첫\" 화");
        meta.synopsis = "줄바꿈\n있는 요약".into();
        meta.target = Some(5000);
        meta.extra.push(("futureKey".into(), "[1, 2]".into()));
        let doc = DocFile {
            meta,
            body: vec![Block::text("본문")],
        };
        let text = write_doc(&doc);
        assert!(text.starts_with("---\nid: \"abc\"\n"));
        assert_eq!(parse_doc(&text, "zzz"), doc);
    }

    #[test]
    fn file_without_front_matter() {
        let doc = parse_doc("그냥 쓴 글\n\n둘째 문단\n", "loose");
        assert_eq!(doc.meta.id, "loose");
        assert_eq!(doc.meta.status, "draft");
        assert_eq!(doc.body.len(), 2);
    }

    #[test]
    fn body_that_starts_with_dashes() {
        let doc = DocFile {
            meta: DocMeta::new("x", "t"),
            body: vec![Block::text("---"), Block::text("본문")],
        };
        assert_eq!(parse_doc(&write_doc(&doc), "x"), doc);
    }
}
