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
//! 본문 (markup/)
//! ```
//!
//! Front matter values are JSON scalars, which YAML also reads. Keys this
//! version does not know are kept as they are.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::count::{Counts, count_blocks};
use crate::layout::PageMetrics;
use crate::markup::{Block, parse_body, write_body};
use crate::project::{MANUSCRIPT_DIR, PLANNING_DIR};
use crate::store::{atomic_write, now_iso, rev_of};
use crate::{Error, Result, journal, project, snapshot};

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

impl DocFile {
    /// Fingerprint of the text (not the title or status). The editor keeps the
    /// one it started from, so a save can tell when the text on disk changed
    /// in the meantime, for example on another device.
    pub fn rev(&self) -> String {
        body_rev(&self.body)
    }
}

pub fn body_rev(body: &[Block]) -> String {
    rev_of(write_body(body).as_bytes())
}

pub(crate) fn decode_value(raw: &str) -> String {
    if raw.starts_with('"') {
        serde_json::from_str::<String>(raw).unwrap_or_else(|_| raw.trim_matches('"').to_string())
    } else {
        raw.to_string()
    }
}

pub(crate) fn encode(value: &str) -> String {
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

pub(crate) fn split_front_matter(src: &str) -> (Option<&str>, &str) {
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
    /// Setting cards (설정집), see cards.rs.
    Cards,
    /// Notes (메모), see notes.rs.
    Notes,
}

impl Section {
    pub fn dir(self) -> &'static str {
        match self {
            Section::Manuscript => MANUSCRIPT_DIR,
            Section::Planning => PLANNING_DIR,
            Section::Cards => crate::cards::CARDS_DIR,
            Section::Notes => crate::notes::NOTES_DIR,
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

/// Why a document file that is there cannot be read as it is
/// (docs/safety-design.md S4). 고쳐 열기 (mend.rs) mends each of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Damage {
    /// Not UTF-8: saved as ANSI (EUC-KR) by Notepad, or as UTF-16.
    Encoding,
    /// Nothing in it, or only blank lines.
    Empty,
    /// The front matter is opened (`---` and `key: value` lines) but never closed.
    FrontMatter,
}

impl Damage {
    /// The reason in the writer's words.
    pub fn message(self) -> &'static str {
        match self {
            Damage::Encoding => "다른 글자 방식(EUC-KR 등)으로 저장된 파일",
            Damage::Empty => "내용이 없는 빈 파일",
            Damage::FrontMatter => "제목 등 회차 정보 부분이 깨진 파일",
        }
    }
}

/// Whether a line looks like a front matter entry (`title: "…"`).
pub(crate) fn is_front_line(line: &str) -> bool {
    let Some((key, _)) = line.split_once(':') else {
        return false;
    };
    let mut chars = key.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// A document file's text (byte order mark dropped, LF line ends), or why
/// it cannot be read as it is.
pub fn check_bytes(bytes: &[u8]) -> std::result::Result<String, Damage> {
    let text = std::str::from_utf8(bytes).map_err(|_| Damage::Encoding)?;
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let text = text.replace("\r\n", "\n");
    if text.trim().is_empty() {
        return Err(Damage::Empty);
    }
    if let Some(rest) = text.strip_prefix("---\n")
        && rest.lines().next().is_some_and(is_front_line)
        && split_front_matter(&text).0.is_none()
    {
        return Err(Damage::FrontMatter);
    }
    Ok(text)
}

/// Reads a document file. One that is empty, in another encoding or with a
/// broken front matter is an error naming the damage (`Damage`), so it is
/// never taken for an empty chapter.
pub fn read_doc(path: &Path) -> Result<DocFile> {
    let fallback = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let bytes = std::fs::read(path).map_err(|e| Error::io(path, e))?;
    let text = check_bytes(&bytes).map_err(|d| Error::format(path, d.message()))?;
    Ok(parse_doc(&text, &fallback))
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
    /// Estimated pages in the project's manuscript format; none without paper.
    pub pages: Option<u32>,
    /// Set when this save also kept a record: an automatic one of the
    /// previous text, or on a conflict the text that could not be saved.
    pub snapshot: Option<snapshot::SnapshotInfo>,
    /// Fingerprint of the text now on disk (see `DocFile::rev`).
    pub rev: String,
    /// The text on disk changed since the editor loaded it (another device
    /// saved it), so nothing was written. The editor's text is kept as a
    /// "this-device" record instead, returned in `snapshot`.
    pub conflict: bool,
}

/// How a save treats text on disk that changed since the editor loaded it.
#[derive(Debug, Clone, Copy, Default)]
pub struct SaveGuard<'a> {
    /// Fingerprint of the text the editor started from. None skips the check.
    pub base: Option<&'a str>,
    /// Save anyway, keeping the text on disk as an "other-device" record.
    pub force: bool,
}

/// Replaces a document's body, keeping its front matter as it is on disk, so
/// that title or status changes made elsewhere are never overwritten by the
/// editor. Keeps an automatic record first when one is due, and a
/// `before-shrink` record whenever the save takes away a lot of the text
/// (`snapshot::shrinks_a_lot`), so a big deletion can always be undone.
///
/// With a `guard.base`, a body on disk that is neither that text nor the one
/// being saved means another device changed it: the save is refused (see
/// `SaveOutcome::conflict`) unless `guard.force` is set.
pub fn save_body(
    root: &Path,
    id: &str,
    body: Vec<Block>,
    auto_record_every: chrono::Duration,
    guard: SaveGuard,
) -> Result<SaveOutcome> {
    let (section, path) = locate(root, id)?;
    let current = read_doc(&path)?;
    let counts = count_blocks(&body);
    let pages = match section {
        Section::Manuscript => project::load(root)
            .ok()
            .and_then(|p| PageMetrics::of(&p.manuscript_format()))
            .map(|m| m.chapter_pages(&body, true)),
        Section::Planning | Section::Cards | Section::Notes => None,
    };
    if current.body == body {
        return Ok(SaveOutcome {
            counts,
            pages,
            snapshot: None,
            rev: current.rev(),
            conflict: false,
        });
    }
    let disk_rev = current.rev();
    let changed_elsewhere = guard.base.is_some_and(|base| base != disk_rev);
    if changed_elsewhere && !guard.force {
        let mine = DocFile {
            meta: current.meta.clone(),
            body,
        };
        let kept = snapshot::keep_unless_same(root, &mine, "this-device")?;
        return Ok(SaveOutcome {
            counts,
            pages,
            snapshot: kept,
            rev: disk_rev,
            conflict: true,
        });
    }
    let before = count_blocks(&current.body).with_spaces;
    let snapshot = if changed_elsewhere {
        snapshot::keep_unless_same(root, &current, "other-device")?
    } else if snapshot::shrinks_a_lot(before, counts.with_spaces) {
        snapshot::keep_unless_same(root, &current, "before-shrink")?
    } else {
        snapshot::auto_if_due(root, &current, auto_record_every)?
    };
    let saved = DocFile {
        meta: current.meta,
        body,
    };
    write_doc_file(&path, &saved)?;
    journal::note(
        root,
        journal::Entry::Save(journal::Save {
            doc: saved.meta.id.clone(),
            body: journal::fingerprint(write_body(&saved.body).as_bytes()),
            chars: counts.with_spaces,
            added: counts.with_spaces.saturating_sub(before),
            removed: before.saturating_sub(counts.with_spaces),
            saves: None,
            since: None,
        }),
    );
    Ok(SaveOutcome {
        counts,
        pages,
        snapshot,
        rev: saved.rev(),
        conflict: false,
    })
}

/// Keeps a record of `body` as the text of document `id`, unless the newest
/// record already has it. Used before the editor loads another device's text.
pub fn keep_record(
    root: &Path,
    id: &str,
    body: Vec<Block>,
    kind: &str,
) -> Result<Option<snapshot::SnapshotInfo>> {
    let (_, path) = locate(root, id)?;
    let current = read_doc(&path)?;
    snapshot::keep_unless_same(
        root,
        &DocFile {
            meta: current.meta,
            body,
        },
        kind,
    )
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
