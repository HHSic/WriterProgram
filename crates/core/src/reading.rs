//! 작품 해석: what sentences of a chapter do and mean, and how they link up
//! (a setup and its payoff, an image that comes back). An assistant writes it
//! through writer-mcp; the app shows it in 해석 모드 when the writer clicks a
//! sentence (docs/reading.md).
//!
//! One file per chapter, `readings/<doc id>.json`. Sentences are kept by
//! their quoted text, not by position, so a reading still finds its sentence
//! after the writer edits around it. Saving checks every quote against the
//! chapter; loading finds each one again in the current text and marks the
//! ones that are gone.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::markup::{Block, inline_text};
use crate::store::{atomic_write, now_iso};
use crate::{Error, Result, doc};

pub const READINGS_DIR: &str = "readings";

/// Suggested roles. Others are kept as they are.
pub const ROLES: [&str; 12] = [
    "복선",
    "회수",
    "인물",
    "관계",
    "긴장",
    "정보",
    "분위기",
    "이미지",
    "리듬",
    "전환",
    "주제",
    "시점",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    /// Id of the other sentence in this reading.
    pub to: String,
    /// How they are linked, e.g. "회수", "같은 이미지", "대비".
    pub kind: String,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentenceReading {
    pub id: String,
    /// Paragraph (index among the chapter's blocks) the quote is in.
    pub block: usize,
    /// The sentence or part of it, exactly as in the text.
    pub quote: String,
    #[serde(default)]
    pub roles: Vec<String>,
    /// What it says beyond its words.
    #[serde(default)]
    pub meaning: String,
    /// Why it is there: what it does for the story.
    #[serde(default)]
    pub intent: String,
    #[serde(default)]
    pub links: Vec<Link>,
    /// Set when the sentence seems to do nothing the text needs, with why.
    /// For the writer to judge, not a verdict.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unclear: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SceneReading {
    /// First paragraph of the scene.
    pub block: usize,
    pub title: String,
    /// What the scene is for in the story.
    pub purpose: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
    pub doc: String,
    /// Fingerprint of the text read (`DocFile::rev`).
    pub rev: String,
    pub made: String,
    /// Who read it, e.g. "Claude".
    #[serde(default)]
    pub by: String,
    /// The chapter in a few lines: what it is about underneath.
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub scenes: Vec<SceneReading>,
    #[serde(default)]
    pub sentences: Vec<SentenceReading>,
}

/// Where a sentence reading sits in the chapter as it is now. Offsets are
/// UTF-16 units in the paragraph's text (line breaks as `\n`), as the editor
/// counts them.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Place {
    pub id: String,
    pub block: usize,
    pub start: usize,
    pub end: usize,
}

/// A reading with each sentence found again in the current text.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingView {
    pub reading: Reading,
    pub places: Vec<Place>,
    /// Sentences whose quote is no longer in the text.
    pub gone: Vec<String>,
    /// The text changed since it was read.
    pub changed: bool,
}

pub fn path(root: &Path, doc_id: &str) -> Result<PathBuf> {
    if doc_id.is_empty() || !doc_id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(Error::Invalid(format!("문서 id가 아님: {doc_id}")));
    }
    Ok(root.join(READINGS_DIR).join(format!("{doc_id}.json")))
}

/// The text of each block: paragraphs as the editor shows them, scene breaks
/// empty.
pub fn block_texts(body: &[Block]) -> Vec<String> {
    body.iter()
        .map(|b| match b {
            Block::Paragraph { content, .. } => inline_text(content),
            Block::SceneBreak {} => String::new(),
        })
        .collect()
}

fn utf16(s: &str) -> usize {
    s.encode_utf16().count()
}

/// Finds `quote` in block `block`, else anywhere in the chapter.
fn find(texts: &[String], block: usize, quote: &str) -> Option<(usize, usize, usize)> {
    let quote = quote.trim();
    if quote.is_empty() {
        return None;
    }
    let at = |b: usize| {
        texts[b].find(quote).map(|i| {
            (
                b,
                utf16(&texts[b][..i]),
                utf16(&texts[b][..i]) + utf16(quote),
            )
        })
    };
    if block < texts.len()
        && let Some(found) = at(block)
    {
        return Some(found);
    }
    (0..texts.len()).find_map(at)
}

/// Checks a reading against the chapter and keeps it. Every quote must be in
/// the text; a quote found in another paragraph than the one given is moved
/// there. Links must point at sentences in the reading.
pub fn save(root: &Path, mut reading: Reading) -> Result<Reading> {
    let current = doc::load(root, &reading.doc)?;
    let texts = block_texts(&current.body);
    let mut missing = Vec::new();
    for s in &mut reading.sentences {
        s.quote = s.quote.trim().to_string();
        match find(&texts, s.block, &s.quote) {
            Some((block, _, _)) => s.block = block,
            None => missing.push(format!("{} \"{}\"", s.id, s.quote)),
        }
    }
    if !missing.is_empty() {
        return Err(Error::Invalid(format!(
            "본문에 없는 인용: {}. 인용은 본문 그대로 옮긴다",
            missing.join(", ")
        )));
    }
    let ids: Vec<&str> = reading.sentences.iter().map(|s| s.id.as_str()).collect();
    let mut seen = std::collections::HashSet::new();
    if let Some(dup) = ids.iter().find(|id| !seen.insert(**id)) {
        return Err(Error::Invalid(format!("같은 id가 두 번 나옴: {dup}")));
    }
    let broken: Vec<String> = reading
        .sentences
        .iter()
        .flat_map(|s| s.links.iter().map(move |l| (s, l)))
        .filter(|(_, l)| !ids.contains(&l.to.as_str()))
        .map(|(s, l)| format!("{}→{}", s.id, l.to))
        .collect();
    if !broken.is_empty() {
        return Err(Error::Invalid(format!(
            "없는 문장을 가리키는 연결: {}",
            broken.join(", ")
        )));
    }
    for scene in &mut reading.scenes {
        scene.block = scene.block.min(texts.len().saturating_sub(1));
    }
    reading.rev = current.rev();
    reading.made = now_iso();
    let path = path(root, &reading.doc)?;
    let dir = root.join(READINGS_DIR);
    fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    let json = serde_json::to_string_pretty(&reading).expect("readings serialize");
    atomic_write(&path, json.as_bytes())?;
    Ok(reading)
}

/// The chapter's reading with every sentence placed in the current text, or
/// none when the chapter has not been read.
pub fn load(root: &Path, doc_id: &str) -> Result<Option<ReadingView>> {
    let path = path(root, doc_id)?;
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(None);
    };
    let reading: Reading = serde_json::from_str(&text).map_err(|e| Error::Format {
        path: path.clone(),
        message: e.to_string(),
    })?;
    let current = doc::load(root, doc_id)?;
    let texts = block_texts(&current.body);
    let mut places = Vec::new();
    let mut gone = Vec::new();
    for s in &reading.sentences {
        match find(&texts, s.block, &s.quote) {
            Some((block, start, end)) => places.push(Place {
                id: s.id.clone(),
                block,
                start,
                end,
            }),
            None => gone.push(s.id.clone()),
        }
    }
    Ok(Some(ReadingView {
        changed: reading.rev != current.rev(),
        reading,
        places,
        gone,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup::parse_body;
    use crate::project::{self, NewProject, ProjectKind};

    fn project_with(body: &str) -> (tempfile::TempDir, PathBuf, String) {
        let dir = tempfile::tempdir().unwrap();
        let root = project::create(&NewProject {
            parent: dir.path().to_str().unwrap().into(),
            title: "해석".into(),
            kind: ProjectKind::Print,
            per_doc_goal: None,
            count_spaces: true,
            first_chapter: true,
            platform: None,
        })
        .unwrap();
        let id = project::load(&root).unwrap().parts[0].docs[0].clone();
        doc::save_body(
            &root,
            &id,
            parse_body(body),
            chrono::Duration::minutes(10),
            Default::default(),
        )
        .unwrap();
        (dir, root, id)
    }

    fn sentence(id: &str, block: usize, quote: &str, links: Vec<Link>) -> SentenceReading {
        SentenceReading {
            id: id.into(),
            block,
            quote: quote.into(),
            roles: vec!["복선".into()],
            meaning: String::new(),
            intent: String::new(),
            links,
            unclear: None,
        }
    }

    fn reading(doc: &str, sentences: Vec<SentenceReading>) -> Reading {
        Reading {
            doc: doc.into(),
            rev: String::new(),
            made: String::new(),
            by: "Claude".into(),
            summary: String::new(),
            scenes: vec![],
            sentences,
        }
    }

    #[test]
    fn quotes_are_checked_and_placed() {
        let (_dir, root, id) =
            project_with("😀 손을 뻗으려다 말았다.\n\n***\n\n손이 머뭇거리지 않았다.");
        let link = Link {
            to: "b".into(),
            kind: "회수".into(),
            note: String::new(),
        };
        // The second quote is given the wrong paragraph; saving moves it.
        let saved = save(
            &root,
            reading(
                &id,
                vec![
                    sentence("a", 0, "손을 뻗으려다 말았다.", vec![link]),
                    sentence("b", 0, "손이 머뭇거리지 않았다.", vec![]),
                ],
            ),
        )
        .unwrap();
        assert_eq!(saved.sentences[1].block, 2);

        let view = load(&root, &id).unwrap().unwrap();
        assert!(!view.changed && view.gone.is_empty());
        // The emoji takes two UTF-16 units.
        assert_eq!(
            view.places[0],
            Place {
                id: "a".into(),
                block: 0,
                start: 3,
                end: 15
            }
        );

        let e = save(
            &root,
            reading(&id, vec![sentence("x", 0, "없는 문장.", vec![])]),
        )
        .unwrap_err();
        assert!(e.to_string().contains("본문에 없는 인용"), "{e}");
        let dangling = Link {
            to: "z".into(),
            kind: "회수".into(),
            note: String::new(),
        };
        let e = save(
            &root,
            reading(
                &id,
                vec![sentence("a", 0, "손을 뻗으려다 말았다.", vec![dangling])],
            ),
        )
        .unwrap_err();
        assert!(e.to_string().contains("a→z"), "{e}");
    }

    #[test]
    fn edits_are_followed_or_marked_gone() {
        let (_dir, root, id) = project_with("첫 문장.\n\n둘째 문장.");
        save(
            &root,
            reading(
                &id,
                vec![
                    sentence("a", 0, "첫 문장.", vec![]),
                    sentence("b", 1, "둘째 문장.", vec![]),
                ],
            ),
        )
        .unwrap();
        doc::save_body(
            &root,
            &id,
            parse_body("새로 넣은 문단.\n\n첫 문장.\n\n고친 문장."),
            chrono::Duration::minutes(10),
            Default::default(),
        )
        .unwrap();
        let view = load(&root, &id).unwrap().unwrap();
        assert!(view.changed);
        assert_eq!(view.places[0].block, 1);
        assert_eq!(view.gone, ["b"]);
        assert!(
            load(&root, "nothing0read").unwrap().is_none() || load(&root, "nothing0read").is_err()
        );
    }
}
