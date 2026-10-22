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
//!
//! A link may reach into another chapter (a setup in chapter 1 paid off in
//! chapter 5): it names that chapter and quotes its sentence, checked
//! against that chapter's text, so the other chapter need not be read yet.
//! Loading a chapter also gathers the links other chapters make into it.
//! The writer can change a reading in the app; that save marks it edited.

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
    /// Id of the other sentence in this reading; empty for a link into
    /// another chapter.
    #[serde(default)]
    pub to: String,
    /// How they are linked, e.g. "회수", "같은 이미지", "대비".
    pub kind: String,
    #[serde(default)]
    pub note: String,
    /// The other chapter, for a link into it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub doc: Option<String>,
    /// The sentence there, exactly as in its text.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub quote: String,
}

impl Link {
    /// The chapter this link reaches into, when it is not `own`.
    pub fn other_doc<'a>(&'a self, own: &str) -> Option<&'a str> {
        self.doc.as_deref().filter(|d| *d != own)
    }
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
    /// When the writer last changed it in the app.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edited: Option<String>,
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

/// A sentence in some chapter, found in its text as it is now.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Spot {
    pub doc: String,
    pub title: String,
    pub block: usize,
    pub start: usize,
    pub end: usize,
}

/// A link of this chapter's reading into another chapter.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Remote {
    /// The sentence here it starts from, and its place among that sentence's links.
    pub from: String,
    pub link: usize,
    pub doc: String,
    pub title: String,
    pub quote: String,
    pub kind: String,
    pub note: String,
    /// Where the quote is now; none when it is gone from that chapter.
    pub spot: Option<Spot>,
}

/// A link another chapter's reading makes into this chapter.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Incoming {
    pub doc: String,
    pub title: String,
    /// The sentence there and its quote.
    pub from: String,
    pub from_quote: String,
    pub from_spot: Option<Spot>,
    pub kind: String,
    pub note: String,
    /// The sentence here and where it is now (its `id` is empty).
    pub quote: String,
    pub place: Option<Place>,
}

/// A reading with each sentence found again in the current text, and the
/// links between it and other chapters.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadingView {
    pub reading: Reading,
    /// This chapter has a reading of its own (not only links into it).
    pub read: bool,
    pub places: Vec<Place>,
    /// Sentences whose quote is no longer in the text.
    pub gone: Vec<String>,
    /// The text changed since it was read.
    pub changed: bool,
    pub remote: Vec<Remote>,
    pub incoming: Vec<Incoming>,
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
/// there. Links must point at sentences in the reading, or quote a sentence
/// of another chapter as it is in that chapter's text.
pub fn save(root: &Path, reading: Reading) -> Result<Reading> {
    keep(root, reading, false)
}

/// Keeps a reading the writer changed in the app. Sentences or links whose
/// quote is no longer in the text stay as they are instead of stopping the
/// save; new ones were taken from the text.
pub fn save_edit(root: &Path, reading: Reading) -> Result<Reading> {
    keep(root, reading, true)
}

fn keep(root: &Path, mut reading: Reading, edit: bool) -> Result<Reading> {
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
    if !missing.is_empty() && !edit {
        return Err(Error::Invalid(format!(
            "본문에 없는 인용: {}. 인용은 본문 그대로 옮긴다",
            missing.join(", ")
        )));
    }
    let ids: Vec<String> = reading.sentences.iter().map(|s| s.id.clone()).collect();
    let mut seen = std::collections::HashSet::new();
    if let Some(dup) = ids.iter().find(|id| !seen.insert(id.as_str())) {
        return Err(Error::Invalid(format!("같은 id가 두 번 나옴: {dup}")));
    }
    let own = reading.doc.clone();
    let mut others: std::collections::HashMap<String, Option<Vec<String>>> = Default::default();
    let mut broken = Vec::new();
    for s in &mut reading.sentences {
        for l in &mut s.links {
            if l.doc.as_deref() == Some(own.as_str()) {
                l.doc = None;
            }
            l.quote = l.quote.trim().to_string();
            match l.doc.clone() {
                None if ids.contains(&l.to) => {}
                None => broken.push(format!("{}→{}", s.id, l.to)),
                Some(other) => {
                    let texts = others.entry(other.clone()).or_insert_with(|| {
                        doc::load(root, &other).ok().map(|d| block_texts(&d.body))
                    });
                    let found = texts.as_ref().and_then(|t| find(t, 0, &l.quote));
                    if found.is_none() && !edit {
                        broken.push(format!("{}→{other} \"{}\"", s.id, l.quote));
                    }
                }
            }
        }
    }
    if !broken.is_empty() {
        return Err(Error::Invalid(format!(
            "없는 문장을 가리키는 연결: {}. 다른 회차는 doc과 그 회차 본문 그대로의 quote로 가리킨다",
            broken.join(", ")
        )));
    }
    for scene in &mut reading.scenes {
        scene.block = scene.block.min(texts.len().saturating_sub(1));
    }
    reading.rev = current.rev();
    if edit {
        reading.edited = Some(now_iso());
    } else {
        reading.made = now_iso();
        reading.edited = None;
    }
    let path = path(root, &reading.doc)?;
    let dir = root.join(READINGS_DIR);
    fs::create_dir_all(&dir).map_err(|e| Error::io(&dir, e))?;
    let json = serde_json::to_string_pretty(&reading).expect("readings serialize");
    atomic_write(&path, json.as_bytes())?;
    Ok(reading)
}

fn read_file(path: &Path) -> Result<Option<Reading>> {
    let Ok(text) = fs::read_to_string(path) else {
        return Ok(None);
    };
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|e| Error::Format {
            path: path.to_path_buf(),
            message: e.to_string(),
        })
}

/// Chapters' titles and texts, read once each.
#[derive(Default)]
struct Texts(std::collections::HashMap<String, Option<(String, Vec<String>)>>);

impl Texts {
    fn of(&mut self, root: &Path, doc_id: &str) -> Option<&(String, Vec<String>)> {
        self.0
            .entry(doc_id.to_string())
            .or_insert_with(|| {
                doc::load(root, doc_id)
                    .ok()
                    .map(|d| (d.meta.title.clone(), block_texts(&d.body)))
            })
            .as_ref()
    }

    fn spot(&mut self, root: &Path, doc_id: &str, quote: &str) -> (String, Option<Spot>) {
        match self.of(root, doc_id) {
            None => (String::new(), None),
            Some((title, texts)) => (
                title.clone(),
                find(texts, 0, quote).map(|(block, start, end)| Spot {
                    doc: doc_id.to_string(),
                    title: title.clone(),
                    block,
                    start,
                    end,
                }),
            ),
        }
    }
}

/// The chapter's reading with every sentence placed in the current text and
/// its links to and from other chapters; none when there is neither.
pub fn load(root: &Path, doc_id: &str) -> Result<Option<ReadingView>> {
    let own = read_file(&path(root, doc_id)?)?;
    let current = doc::load(root, doc_id)?;
    let mut texts = Texts::default();
    let here = block_texts(&current.body);

    let mut incoming = Vec::new();
    if let Ok(entries) = fs::read_dir(root.join(READINGS_DIR)) {
        let mut files: Vec<PathBuf> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        files.sort();
        for file in files {
            let Some(stem) = file.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if stem == doc_id || file.extension().is_none_or(|e| e != "json") {
                continue;
            }
            // Another chapter's reading that cannot be read only loses its links here.
            let Ok(Some(other)) = read_file(&file) else {
                continue;
            };
            for s in &other.sentences {
                for l in s
                    .links
                    .iter()
                    .filter(|l| l.other_doc(&other.doc) == Some(doc_id))
                {
                    let (title, from_spot) = texts.spot(root, &other.doc, &s.quote);
                    let place = find(&here, 0, &l.quote).map(|(block, start, end)| Place {
                        id: String::new(),
                        block,
                        start,
                        end,
                    });
                    incoming.push(Incoming {
                        doc: other.doc.clone(),
                        title,
                        from: s.id.clone(),
                        from_quote: s.quote.clone(),
                        from_spot,
                        kind: l.kind.clone(),
                        note: l.note.clone(),
                        quote: l.quote.clone(),
                        place,
                    });
                }
            }
        }
    }

    let read = own.is_some();
    if !read && incoming.is_empty() {
        return Ok(None);
    }
    let reading = own.unwrap_or_else(|| Reading {
        doc: doc_id.to_string(),
        rev: current.rev(),
        made: String::new(),
        by: String::new(),
        summary: String::new(),
        scenes: Vec::new(),
        sentences: Vec::new(),
        edited: None,
    });
    let mut places = Vec::new();
    let mut gone = Vec::new();
    let mut remote = Vec::new();
    for s in &reading.sentences {
        match find(&here, s.block, &s.quote) {
            Some((block, start, end)) => places.push(Place {
                id: s.id.clone(),
                block,
                start,
                end,
            }),
            None => gone.push(s.id.clone()),
        }
        for (i, l) in s.links.iter().enumerate() {
            if let Some(other) = l.other_doc(doc_id) {
                let (title, spot) = texts.spot(root, other, &l.quote);
                remote.push(Remote {
                    from: s.id.clone(),
                    link: i,
                    doc: other.to_string(),
                    title,
                    quote: l.quote.clone(),
                    kind: l.kind.clone(),
                    note: l.note.clone(),
                    spot,
                });
            }
        }
    }
    Ok(Some(ReadingView {
        changed: read && reading.rev != current.rev(),
        read,
        reading,
        places,
        gone,
        remote,
        incoming,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup::parse_body;
    use crate::project::{self, NewDoc, NewProject, ProjectKind};

    fn put(root: &Path, id: &str, body: &str) {
        doc::save_body(
            root,
            id,
            parse_body(body),
            chrono::Duration::minutes(10),
            Default::default(),
        )
        .unwrap();
    }

    /// A project whose chapters hold `bodies`; their ids in order.
    fn project_with(bodies: &[&str]) -> (tempfile::TempDir, PathBuf, Vec<String>) {
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
        let mut ids = vec![project::load(&root).unwrap().parts[0].docs[0].clone()];
        for _ in 1..bodies.len() {
            ids.push(project::add_doc(&root, &NewDoc::default()).unwrap());
        }
        for (id, body) in ids.iter().zip(bodies) {
            put(&root, id, body);
        }
        (dir, root, ids)
    }

    fn link(to: &str) -> Link {
        Link {
            to: to.into(),
            kind: "회수".into(),
            note: String::new(),
            doc: None,
            quote: String::new(),
        }
    }

    fn far(doc: &str, quote: &str) -> Link {
        Link {
            to: String::new(),
            kind: "회수".into(),
            note: "다른 회차".into(),
            doc: Some(doc.into()),
            quote: quote.into(),
        }
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
            edited: None,
        }
    }

    #[test]
    fn quotes_are_checked_and_placed() {
        let (_dir, root, ids) =
            project_with(&["😀 손을 뻗으려다 말았다.\n\n***\n\n손이 머뭇거리지 않았다."]);
        let id = &ids[0];
        // The second quote is given the wrong paragraph; saving moves it.
        let saved = save(
            &root,
            reading(
                id,
                vec![
                    sentence("a", 0, "손을 뻗으려다 말았다.", vec![link("b")]),
                    sentence("b", 0, "손이 머뭇거리지 않았다.", vec![]),
                ],
            ),
        )
        .unwrap();
        assert_eq!(saved.sentences[1].block, 2);

        let view = load(&root, id).unwrap().unwrap();
        assert!(view.read && !view.changed && view.gone.is_empty());
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
            reading(id, vec![sentence("x", 0, "없는 문장.", vec![])]),
        )
        .unwrap_err();
        assert!(e.to_string().contains("본문에 없는 인용"), "{e}");
        let e = save(
            &root,
            reading(
                id,
                vec![sentence("a", 0, "손을 뻗으려다 말았다.", vec![link("z")])],
            ),
        )
        .unwrap_err();
        assert!(e.to_string().contains("a→z"), "{e}");
    }

    #[test]
    fn edits_are_followed_or_marked_gone() {
        let (_dir, root, ids) = project_with(&["첫 문장.\n\n둘째 문장."]);
        let id = &ids[0];
        save(
            &root,
            reading(
                id,
                vec![
                    sentence("a", 0, "첫 문장.", vec![]),
                    sentence("b", 1, "둘째 문장.", vec![]),
                ],
            ),
        )
        .unwrap();
        put(&root, id, "새로 넣은 문단.\n\n첫 문장.\n\n고친 문장.");
        let view = load(&root, id).unwrap().unwrap();
        assert!(view.changed);
        assert_eq!(view.places[0].block, 1);
        assert_eq!(view.gone, ["b"]);
    }

    #[test]
    fn links_reach_into_other_chapters_both_ways() {
        let (_dir, root, ids) = project_with(&[
            "남의 대문에 손을 대 본 적은 한 번도 없었다.",
            "비가 왔다.\n\n손이 머뭇거리지 않았다.",
        ]);
        let (one, five) = (&ids[0], &ids[1]);
        // Chapter 5 has no reading yet; the quote is checked against its text.
        let e = save(
            &root,
            reading(
                one,
                vec![sentence(
                    "s1",
                    0,
                    "남의 대문에",
                    vec![far(five, "지어낸 문장.")],
                )],
            ),
        )
        .unwrap_err();
        assert!(e.to_string().contains("s1→"), "{e}");
        save(
            &root,
            reading(
                one,
                vec![sentence(
                    "s1",
                    0,
                    "남의 대문에 손을 대 본 적은 한 번도 없었다.",
                    vec![far(five, "손이 머뭇거리지 않았다.")],
                )],
            ),
        )
        .unwrap();

        let from = load(&root, one).unwrap().unwrap();
        assert_eq!(from.remote.len(), 1);
        let spot = from.remote[0].spot.as_ref().unwrap();
        assert_eq!(
            (spot.doc.as_str(), spot.block, spot.start),
            (five.as_str(), 1, 0)
        );

        // Chapter 5 shows the link coming in, though it has no reading of its own.
        let to = load(&root, five).unwrap().unwrap();
        assert!(!to.read);
        assert_eq!(to.incoming.len(), 1);
        let came = &to.incoming[0];
        assert_eq!(
            (came.doc.as_str(), came.from.as_str()),
            (one.as_str(), "s1")
        );
        assert_eq!(came.place.as_ref().unwrap().block, 1);
        assert_eq!(came.from_spot.as_ref().unwrap().block, 0);
    }

    #[test]
    fn the_writers_edits_keep_what_is_gone_and_are_marked() {
        let (_dir, root, ids) = project_with(&["첫 문장.\n\n둘째 문장."]);
        let id = &ids[0];
        save(
            &root,
            reading(
                id,
                vec![
                    sentence("a", 0, "첫 문장.", vec![]),
                    sentence("b", 1, "둘째 문장.", vec![]),
                ],
            ),
        )
        .unwrap();
        put(&root, id, "첫 문장.\n\n고친 문장.");
        let mut r = load(&root, id).unwrap().unwrap().reading;
        r.sentences[0].meaning = "작가가 고친 뜻".into();
        let kept = save_edit(&root, r).unwrap();
        assert!(kept.edited.is_some());
        assert_eq!(kept.sentences.len(), 2);
        let view = load(&root, id).unwrap().unwrap();
        assert_eq!(view.reading.sentences[0].meaning, "작가가 고친 뜻");
        assert_eq!(view.gone, ["b"]);
        assert!(!view.changed);
    }
}
