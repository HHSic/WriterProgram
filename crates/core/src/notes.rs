//! 메모 (docs/layout-data.md "메모"): notes on a stretch of text, a chapter, a
//! setting card, or the whole project, with replies, tags and 완료.
//!
//! One note per file in `notes/<id>.md`, readable like a manuscript file:
//!
//! ```text
//! ---
//! id: "k7q2m9x4t1ab"
//! on: "text"
//! target: "d1k2…"
//! quote: "영업, 끝났나요?"
//! tags: ["퇴고"]
//! done: false
//! replies: [{"at":"2026-09-27T02:00:00.000Z","text":"12화에서 다시 보기"}]
//! created: "2026-09-27T01:00:00.000Z"
//! updated: "2026-09-27T02:00:00.000Z"
//! ---
//!
//! 이 대사 너무 설명조
//! ```
//!
//! A note on a stretch of text is also marked in the chapter file with
//! `<mark data-memo="id">…</mark>`, so it follows the text as it is edited.
//! When all of the marked text is deleted the note stays with the chapter,
//! showing the text it was on (`quote`).

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::doc::Section;
use crate::doc::{decode_value, encode, split_front_matter};
use crate::markup::{Block, Inline, Mark, parse_body, write_body};
use crate::store::{atomic_write, new_id, now_iso, read_text};
use crate::{Error, Result, cards, copies, doc};

pub const NOTES_DIR: &str = "notes";

/// What a note hangs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    /// A stretch of text in a chapter or planning document.
    Text,
    /// A chapter or planning document as a whole.
    Doc,
    Card,
    /// The project: nowhere in particular (작품 메모).
    Project,
}

impl Anchor {
    fn as_str(self) -> &'static str {
        match self {
            Anchor::Text => "text",
            Anchor::Doc => "doc",
            Anchor::Card => "card",
            Anchor::Project => "project",
        }
    }

    fn parse(s: &str) -> Anchor {
        match s {
            "text" => Anchor::Text,
            "doc" => Anchor::Doc,
            "card" => Anchor::Card,
            _ => Anchor::Project,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reply {
    pub at: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub id: String,
    pub anchor: Anchor,
    /// The document (text, doc) or card (card) it is on; empty for the project.
    #[serde(default)]
    pub target: String,
    /// The marked text, as it was when the note was last saved.
    #[serde(default)]
    pub quote: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub replies: Vec<Reply>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub created: String,
    #[serde(default)]
    pub updated: String,
    /// Front matter lines this version does not know.
    #[serde(skip)]
    pub extra: Vec<(String, String)>,
}

/// A note to make. `id` may be chosen by the editor, which marks the text
/// before the note is saved.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewNote {
    #[serde(default)]
    pub id: Option<String>,
    pub anchor: Anchor,
    #[serde(default)]
    pub target: String,
    #[serde(default)]
    pub quote: String,
    #[serde(default)]
    pub text: String,
}

fn text_blocks(text: &str) -> Vec<Block> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    text.split('\n').map(Block::text).collect()
}

fn blocks_text(blocks: &[Block]) -> String {
    blocks
        .iter()
        .map(|b| b.lines().join(" "))
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn parse_note(src: &str, fallback_id: &str) -> Note {
    let (front, body) = split_front_matter(src);
    let mut note = Note {
        id: fallback_id.into(),
        anchor: Anchor::Project,
        target: String::new(),
        quote: String::new(),
        text: blocks_text(&parse_body(body)),
        replies: Vec::new(),
        tags: Vec::new(),
        done: false,
        created: String::new(),
        updated: String::new(),
        extra: Vec::new(),
    };
    for line in front.unwrap_or_default().lines() {
        let Some((key, raw)) = line.split_once(':') else {
            continue;
        };
        let (key, raw) = (key.trim(), raw.trim());
        match key {
            "id" => note.id = decode_value(raw),
            "on" => note.anchor = Anchor::parse(&decode_value(raw)),
            "target" => note.target = decode_value(raw),
            "quote" => note.quote = decode_value(raw),
            "tags" => note.tags = serde_json::from_str(raw).unwrap_or_default(),
            "done" => note.done = raw == "true",
            "replies" => note.replies = serde_json::from_str(raw).unwrap_or_default(),
            "created" => note.created = decode_value(raw),
            "updated" => note.updated = decode_value(raw),
            _ => note.extra.push((key.into(), raw.into())),
        }
    }
    if note.id.is_empty() {
        note.id = fallback_id.into();
    }
    note
}

pub fn write_note(note: &Note) -> String {
    let mut out = String::from("---\n");
    out.push_str(&format!("id: {}\n", encode(&note.id)));
    out.push_str(&format!("on: {}\n", encode(note.anchor.as_str())));
    if !note.target.is_empty() {
        out.push_str(&format!("target: {}\n", encode(&note.target)));
    }
    if !note.quote.is_empty() {
        out.push_str(&format!("quote: {}\n", encode(&note.quote)));
    }
    out.push_str(&format!(
        "tags: {}\n",
        serde_json::to_string(&note.tags).expect("tags serialize")
    ));
    out.push_str(&format!("done: {}\n", note.done));
    out.push_str(&format!(
        "replies: {}\n",
        serde_json::to_string(&note.replies).expect("replies serialize")
    ));
    out.push_str(&format!("created: {}\n", encode(&note.created)));
    out.push_str(&format!("updated: {}\n", encode(&note.updated)));
    for (key, raw) in &note.extra {
        out.push_str(&format!("{key}: {raw}\n"));
    }
    out.push_str("---\n\n");
    out.push_str(&write_body(&text_blocks(&note.text)));
    out
}

fn check_id(id: &str) -> Result<()> {
    let ok = !id.is_empty()
        && id.len() <= 40
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid("올바르지 않은 메모".into()))
    }
}

pub(crate) fn path(root: &Path, id: &str) -> Result<PathBuf> {
    check_id(id)?;
    Ok(root.join(NOTES_DIR).join(doc::file_name(id)))
}

pub fn load(root: &Path, id: &str) -> Result<Note> {
    let path = path(root, id)?;
    if !path.is_file() {
        return Err(Error::NotFound("메모를 찾을 수 없음".into()));
    }
    Ok(parse_note(&read_text(&path)?, id))
}

/// Every note, oldest first. Copies left by sync programs are not notes of
/// their own (copies.rs).
pub fn list(root: &Path) -> Result<Vec<Note>> {
    let dir = root.join(NOTES_DIR);
    let mut notes = Vec::new();
    for id in copies::scan(root, Section::Notes)?.ids {
        // A file a sync program is still writing is left out for now.
        let Ok(text) = read_text(&dir.join(doc::file_name(&id))) else {
            continue;
        };
        notes.push(parse_note(&text, &id));
    }
    notes.sort_by(|a, b| a.created.cmp(&b.created).then_with(|| a.id.cmp(&b.id)));
    Ok(notes)
}

fn clean_tags(tags: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for tag in tags {
        let tag = tag.trim().trim_start_matches('#').trim();
        if !tag.is_empty() && !out.iter().any(|t| t == tag) {
            out.push(tag.to_string());
        }
    }
    out
}

fn check_target(root: &Path, anchor: Anchor, target: &str) -> Result<()> {
    match anchor {
        Anchor::Project => Ok(()),
        Anchor::Text | Anchor::Doc => doc::locate(root, target).map(|_| ()),
        Anchor::Card => cards::load(root, target).map(|_| ()),
    }
}

pub fn create(root: &Path, spec: &NewNote) -> Result<Note> {
    let target = if spec.anchor == Anchor::Project {
        String::new()
    } else {
        spec.target.trim().to_string()
    };
    check_target(root, spec.anchor, &target)?;
    let dir = root.join(NOTES_DIR);
    let id = match &spec.id {
        Some(id) => {
            if path(root, id)?.exists() {
                return Err(Error::Invalid("같은 메모가 이미 있음".into()));
            }
            id.clone()
        }
        None => loop {
            let id = new_id();
            if !dir.join(doc::file_name(&id)).exists() {
                break id;
            }
        },
    };
    let now = now_iso();
    let note = Note {
        id,
        anchor: spec.anchor,
        target,
        quote: spec.quote.chars().take(200).collect(),
        text: spec.text.clone(),
        replies: Vec::new(),
        tags: Vec::new(),
        done: false,
        created: now.clone(),
        updated: now,
        extra: Vec::new(),
    };
    fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    atomic_write(&path(root, &note.id)?, write_note(&note).as_bytes())?;
    Ok(note)
}

/// Saves a note as edited on screen. What it hangs on and when it was made
/// stay as they are on disk.
pub fn save(root: &Path, note: &Note) -> Result<Note> {
    let path = path(root, &note.id)?;
    let current = load(root, &note.id)?;
    let saved = Note {
        id: current.id,
        anchor: current.anchor,
        target: current.target,
        quote: note.quote.chars().take(200).collect(),
        text: note.text.clone(),
        replies: note
            .replies
            .iter()
            .filter(|r| !r.text.trim().is_empty())
            .cloned()
            .collect(),
        tags: clean_tags(&note.tags),
        done: note.done,
        created: current.created,
        updated: now_iso(),
        extra: current.extra,
    };
    atomic_write(&path, write_note(&saved).as_bytes())?;
    Ok(saved)
}

// ---------------------------------------------------------------------------
// Marks in the text

/// Takes the marks of note `id` out of a body. True when something changed.
pub fn unmark(body: &mut [Block], id: &str) -> bool {
    let mut changed = false;
    for block in body.iter_mut() {
        let Block::Paragraph { content, .. } = block else {
            continue;
        };
        for inline in content.iter_mut() {
            if let Inline::Text { marks, .. } = inline {
                let before = marks.len();
                marks.retain(|m| !matches!(m, Mark::Memo { attrs } if attrs.id == id));
                changed |= marks.len() != before;
            }
        }
        // Neighbours that now look the same become one run again.
        let mut merged: Vec<Inline> = Vec::with_capacity(content.len());
        for inline in content.drain(..) {
            match (merged.last_mut(), inline) {
                (
                    Some(Inline::Text { text, marks }),
                    Inline::Text {
                        text: next,
                        marks: next_marks,
                    },
                ) if *marks == next_marks => text.push_str(&next),
                (_, inline) => merged.push(inline),
            }
        }
        *content = merged;
    }
    changed
}

/// Takes a note's marks out of its document file (when the note goes to the trash).
pub fn unmark_file(root: &Path, note: &Note) -> Result<()> {
    if note.anchor != Anchor::Text {
        return Ok(());
    }
    let Ok((_, path)) = doc::locate(root, &note.target) else {
        return Ok(());
    };
    let mut file = doc::read_doc(&path)?;
    if unmark(&mut file.body, &note.id) {
        doc::write_doc_file(&path, &file)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup::MemoAttrs;

    fn memo(id: &str) -> Mark {
        Mark::Memo {
            attrs: MemoAttrs { id: id.into() },
        }
    }

    fn t(text: &str, marks: &[Mark]) -> Inline {
        Inline::Text {
            text: text.into(),
            marks: marks.to_vec(),
        }
    }

    #[test]
    fn round_trip() {
        let note = Note {
            id: "n1".into(),
            anchor: Anchor::Text,
            target: "d1".into(),
            quote: "“영업, 끝났나요?”".into(),
            text: "이 대사 너무 설명조\n\n*별표*도 그대로".into(),
            replies: vec![Reply {
                at: "2026-09-27T02:00:00.000Z".into(),
                text: "12화에서\n다시 보기".into(),
            }],
            tags: vec!["퇴고".into(), "할 일".into()],
            done: true,
            created: "2026-09-27T01:00:00.000Z".into(),
            updated: "2026-09-27T02:00:00.000Z".into(),
            extra: vec![("color".into(), "\"amber\"".into())],
        };
        let text = write_note(&note);
        assert!(text.contains("on: \"text\"\n"));
        assert_eq!(parse_note(&text, "x"), note);
    }

    #[test]
    fn project_note_has_no_target_line() {
        let mut note = parse_note("", "n2");
        note.text = "떠오른 장면".into();
        let text = write_note(&note);
        assert!(!text.contains("target:"));
        assert_eq!(parse_note(&text, "n2").anchor, Anchor::Project);
    }

    #[test]
    fn unmark_merges_runs() {
        let b = Mark::Bold {};
        let mut body = vec![Block::Paragraph {
            attrs: Default::default(),
            content: vec![
                t("앞 ", &[]),
                t("구간", &[memo("a")]),
                t("겹침", &[memo("a"), memo("b")]),
                t(" 뒤", &[]),
                t("굵게", std::slice::from_ref(&b)),
            ],
        }];
        assert!(unmark(&mut body, "a"));
        assert_eq!(
            body,
            vec![Block::Paragraph {
                attrs: Default::default(),
                content: vec![
                    t("앞 구간", &[]),
                    t("겹침", &[memo("b")]),
                    t(" 뒤", &[]),
                    t("굵게", &[b])
                ],
            }]
        );
        assert!(!unmark(&mut body, "a"));
    }
}
