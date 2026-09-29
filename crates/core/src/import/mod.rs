//! 가져오기: txt, md and docx files become chapters.
//!
//! A file is read into a flat list of [`Item`]s (paragraphs, scene breaks,
//! headings, and marks of what cannot be imported). The items are then split
//! into chapters by a rule (`SplitRule`). Tables, pictures and footnotes are
//! never imported: they are counted, and the screen tells the writer before
//! anything is made. With `leave_notes`, each place where something was left
//! out gets a 메모 so it can be found again.
//!
//! The screen calls [`preview`] as often as the writer changes an option, and
//! [`commit`] once. `commit` reads the files again with the same options, so
//! the chapters it makes are the ones the preview listed.

mod text;
mod word;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::count::count_blocks;
use crate::doc::{self, DocFile, DocMeta, STATUSES};
use crate::markup::{Block, Inline, Mark, MemoAttrs, plain_text};
use crate::notes::{self, Anchor, NewNote};
use crate::project::{self, MANUSCRIPT_DIR};
use crate::store::new_id;
use crate::{Error, Result};

/// Where to cut a file into chapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SplitRule {
    /// Whichever of the rules below fits the file best.
    #[default]
    Auto,
    /// The whole file is one chapter.
    File,
    /// Heading styles in docx, `#` lines in md.
    Heading,
    /// "제3화", "3화", "3회".
    Episode,
    /// "제3장", "3장", "Chapter 3".
    Chapter,
    /// "#3".
    Number,
    /// The writer's own pattern (`ImportOptions::pattern`).
    Regex,
}

/// How line breaks in txt and md read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LineMode {
    /// Line breaks inside a block of lines are kept when empty lines separate
    /// paragraphs, and every line is a paragraph otherwise.
    #[default]
    Auto,
    /// Every line is a paragraph.
    Line,
    /// Empty lines separate paragraphs; a line break inside one stays a line break.
    Blank,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ImportOptions {
    pub rule: SplitRule,
    /// Used by `SplitRule::Regex`.
    pub pattern: String,
    pub line_mode: LineMode,
    /// `utf-8` or `euc-kr` for txt and md; found out when missing.
    pub encoding: Option<String>,
}

/// What is left out of an import.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum SkipKind {
    Table,
    Image,
    Footnote,
}

impl SkipKind {
    fn label(self) -> &'static str {
        match self {
            SkipKind::Table => "표",
            SkipKind::Image => "그림",
            SkipKind::Footnote => "각주",
        }
    }
}

/// One thing read from a file, in order.
pub(crate) enum Item {
    Para(Vec<Inline>),
    Scene,
    Heading { level: u8, text: String },
    Skip(SkipKind, u32),
}

/// What a file read gave.
pub(crate) struct Raw {
    pub items: Vec<Item>,
    /// txt and md only.
    pub encoding: Option<String>,
}

/// A place in a chapter where something was left out: after the first `at`
/// paragraphs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skip {
    pub at: usize,
    pub kind: SkipKind,
    pub count: u32,
}

#[derive(Debug, Clone)]
pub struct Chapter {
    /// Index into the files given.
    pub file: usize,
    pub title: String,
    pub body: Vec<Block>,
    pub skips: Vec<Skip>,
}

// ---------------------------------------------------------------------------
// Title patterns

const AFTER_NUMBER: &str = r"(?:$|[\s.:·\-—)\]])";

static EPISODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^(?:제\s*)?\d+\s*[화회]{AFTER_NUMBER}")).unwrap());
static CHAPTER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^(?:(?:제\s*)?\d+\s*장|(?i:chapter|ch)\.?\s*\d+){AFTER_NUMBER}"
    ))
    .unwrap()
});
static NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^#\s*\d+{AFTER_NUMBER}")).unwrap());
/// The number in front of a title, which the app numbers by itself.
static NUMBER_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:(?:제\s*)?\d+\s*[화회장]|(?i:chapter|ch)\.?\s*\d+|#\s*\d+)").unwrap()
});

/// Longest line that can be a chapter title.
const TITLE_MAX_CHARS: usize = 60;
const TITLE_KEEP_CHARS: usize = 100;

fn is_separator(c: char) -> bool {
    c.is_whitespace() || matches!(c, '.' | ':' | '·' | '-' | '—' | ')' | ']')
}

/// "제3화 비에 젖은 손님" is titled "비에 젖은 손님": chapters are numbered by
/// their place in the tree. A line with nothing but the number keeps it.
fn clean_title(line: &str) -> String {
    let line = line.trim();
    let rest = NUMBER_PREFIX
        .find(line)
        .map(|m| line[m.end()..].trim_start_matches(is_separator).trim_end())
        .filter(|rest| !rest.is_empty())
        .unwrap_or(line);
    rest.chars().take(TITLE_KEEP_CHARS).collect()
}

/// Ways of telling a chapter's title line.
#[derive(Clone)]
enum Split {
    File,
    Heading(u8),
    Pattern { re: Regex, custom: bool },
}

impl Split {
    fn label(&self, rule: SplitRule) -> &'static str {
        match (self, rule) {
            (Split::File, _) => "파일 하나가 회차 하나",
            (Split::Heading(_), _) => "제목 서식",
            (Split::Pattern { custom: true, .. }, _) => "직접 쓴 패턴",
            (Split::Pattern { re, .. }, _) if re.as_str() == EPISODE.as_str() => "제N화",
            (Split::Pattern { re, .. }, _) if re.as_str() == CHAPTER.as_str() => {
                "제N장 · Chapter N"
            }
            _ => "#N",
        }
    }
}

fn plain(inlines: &[Inline]) -> String {
    let mut s = String::new();
    for i in inlines {
        match i {
            Inline::Text { text, .. } => s.push_str(text),
            Inline::HardBreak {} => s.push('\n'),
        }
    }
    s
}

/// The first line of a paragraph and what follows it.
fn split_first_line(inlines: &[Inline]) -> (String, Vec<Inline>) {
    let cut = inlines
        .iter()
        .position(|i| matches!(i, Inline::HardBreak {}))
        .unwrap_or(inlines.len());
    let rest = inlines
        .get(cut + 1..)
        .map(<[Inline]>::to_vec)
        .unwrap_or_default();
    (plain(&inlines[..cut]), rest)
}

fn is_title_line(re: &Regex, line: &str) -> bool {
    let line = line.trim();
    line.chars().count() <= TITLE_MAX_CHARS && re.is_match(line)
}

/// Headings become plain paragraphs, for rules that do not cut at them.
fn demote(item: Item) -> Item {
    match item {
        Item::Heading { text, .. } => Item::Para(vec![Inline::Text {
            text,
            marks: vec![],
        }]),
        other => other,
    }
}

/// How many title lines `re` finds, and whether the first thing in the file is one.
fn count_pattern(items: &[Item], re: &Regex) -> (usize, bool) {
    let mut count = 0;
    let mut first = None;
    for item in items {
        let line = match item {
            Item::Para(inl) => split_first_line(inl).0,
            Item::Heading { text, .. } => text.clone(),
            _ => continue,
        };
        let hit = is_title_line(re, &line);
        first.get_or_insert(hit);
        count += usize::from(hit);
    }
    (count, first == Some(true))
}

fn fits(count: usize, first: bool) -> bool {
    count >= 2 || (count == 1 && first)
}

fn resolve(items: &[Item], opts: &ImportOptions) -> std::result::Result<Split, String> {
    let heading = |items: &[Item]| {
        let min = items
            .iter()
            .filter_map(|i| match i {
                Item::Heading { level, .. } => Some(*level),
                _ => None,
            })
            .min()?;
        let count = items
            .iter()
            .filter(|i| matches!(i, Item::Heading { level, .. } if *level == min))
            .count();
        let first = items
            .iter()
            .find(|i| !matches!(i, Item::Skip(..)))
            .is_some_and(|i| matches!(i, Item::Heading { level, .. } if *level == min));
        Some((min, count, first))
    };
    let pattern = |re: &LazyLock<Regex>| Split::Pattern {
        re: Regex::clone(re),
        custom: false,
    };
    Ok(match opts.rule {
        SplitRule::File => Split::File,
        SplitRule::Heading => heading(items).map_or(Split::File, |(min, ..)| Split::Heading(min)),
        SplitRule::Episode => pattern(&EPISODE),
        SplitRule::Chapter => pattern(&CHAPTER),
        SplitRule::Number => pattern(&NUMBER),
        SplitRule::Regex => {
            let re = Regex::new(opts.pattern.trim())
                .map_err(|_| "직접 쓴 패턴이 올바르지 않음".to_string())?;
            if opts.pattern.trim().is_empty() {
                return Err("직접 쓴 패턴이 비어 있음".into());
            }
            Split::Pattern { re, custom: true }
        }
        SplitRule::Auto => {
            if let Some((min, ..)) = heading(items).filter(|(_, c, f)| fits(*c, *f)) {
                return Ok(Split::Heading(min));
            }
            [&EPISODE, &CHAPTER, &NUMBER]
                .into_iter()
                .find(|re| {
                    let (count, first) = count_pattern(items, re);
                    fits(count, first)
                })
                .map_or(Split::File, pattern)
        }
    })
}

fn push_skip(skips: &mut Vec<Skip>, at: usize, kind: SkipKind, count: u32) {
    match skips.last_mut() {
        Some(last) if last.at == at && last.kind == kind => last.count += count,
        _ => skips.push(Skip { at, kind, count }),
    }
}

/// Cuts a file's items into chapters. The part before the first title, if
/// there is one, is a chapter named after the file.
fn split(items: Vec<Item>, how: &Split, stem: &str, file: usize) -> Vec<Chapter> {
    let items: Vec<Item> = match how {
        Split::File => items.into_iter().map(demote).collect(),
        Split::Heading(level) => items
            .into_iter()
            .map(|i| match i {
                Item::Heading { level: l, .. } if l != *level => demote(i),
                other => other,
            })
            .collect(),
        Split::Pattern { re, .. } => {
            let mut out = Vec::with_capacity(items.len());
            for item in items.into_iter().map(demote) {
                match item {
                    Item::Para(inl) => {
                        let (line, rest) = split_first_line(&inl);
                        if is_title_line(re, &line) {
                            out.push(Item::Heading {
                                level: 1,
                                text: line.trim().to_string(),
                            });
                            if !plain(&rest).trim().is_empty() {
                                out.push(Item::Para(rest));
                            }
                        } else {
                            out.push(Item::Para(inl));
                        }
                    }
                    other => out.push(other),
                }
            }
            out
        }
    };

    let new = |title: String| Chapter {
        file,
        title,
        body: Vec::new(),
        skips: Vec::new(),
    };
    let mut lead = new(stem.chars().take(TITLE_KEEP_CHARS).collect());
    let mut titled: Vec<Chapter> = Vec::new();
    for item in items {
        if let Item::Heading { text, .. } = &item {
            let title = match how {
                Split::Pattern { re, custom: true } => re
                    .captures(text)
                    .and_then(|c| c.get(1))
                    .map_or(text.trim(), |m| m.as_str().trim())
                    .chars()
                    .take(TITLE_KEEP_CHARS)
                    .collect(),
                _ => clean_title(text),
            };
            titled.push(new(title));
            continue;
        }
        let chapter = titled.last_mut().unwrap_or(&mut lead);
        match item {
            Item::Para(inl) => chapter.body.push(Block::para(inl)),
            Item::Scene => chapter.body.push(Block::SceneBreak {}),
            Item::Skip(kind, count) => {
                let at = chapter.body.len();
                push_skip(&mut chapter.skips, at, kind, count);
            }
            Item::Heading { .. } => unreachable!("handled above"),
        }
    }
    if titled.is_empty() {
        if lead.body.is_empty() && lead.skips.is_empty() {
            return Vec::new();
        }
        return vec![lead];
    }
    if !lead.body.is_empty() || !lead.skips.is_empty() {
        titled.insert(0, lead);
    }
    titled
}

// ---------------------------------------------------------------------------
// Reading files

fn read_raw(path: &Path, opts: &ImportOptions) -> std::result::Result<Raw, String> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let read = || fs::read(path).map_err(|e| Error::io(path, e).user_message());
    match ext.as_str() {
        "txt" => text::read(&read()?, opts, false),
        "md" | "markdown" => text::read(&read()?, opts, true),
        "docx" => word::read(&read()?),
        "doc" => Err("옛 Word 형식(.doc)은 읽을 수 없음 · Word에서 docx로 저장해 주세요".into()),
        "hwp" | "hwpx" => {
            Err("한글 파일은 아직 읽을 수 없음 · 한글에서 docx나 txt로 저장해 주세요".into())
        }
        _ => Err("가져올 수 없는 파일 형식 (txt, md, docx)".into()),
    }
}

/// What became of one file.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileInfo {
    pub name: String,
    /// txt, md or docx.
    pub kind: String,
    /// Why the file cannot be imported.
    pub error: Option<String>,
    /// How the text was read (txt and md).
    pub encoding: Option<String>,
    /// The way the file was cut into chapters, in screen words.
    pub rule: String,
    pub chapters: usize,
}

fn read_files(paths: &[PathBuf], opts: &ImportOptions) -> (Vec<FileInfo>, Vec<Chapter>) {
    let mut infos = Vec::new();
    let mut chapters = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let stem = path
            .file_stem()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let kind = path
            .extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let mut info = FileInfo {
            name,
            kind,
            error: None,
            encoding: None,
            rule: String::new(),
            chapters: 0,
        };
        let result = read_raw(path, opts).and_then(|raw| {
            let how = resolve(&raw.items, opts)?;
            info.encoding = raw.encoding;
            info.rule = how.label(opts.rule).into();
            let made = split(raw.items, &how, &stem, index);
            if made.is_empty() {
                return Err("가져올 내용이 없음".into());
            }
            Ok(made)
        });
        match result {
            Ok(made) => {
                info.chapters = made.len();
                chapters.extend(made);
            }
            Err(e) => info.error = Some(e),
        }
        infos.push(info);
    }
    (infos, chapters)
}

// ---------------------------------------------------------------------------
// Preview

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct SkipTotals {
    pub tables: u32,
    pub images: u32,
    pub footnotes: u32,
}

impl SkipTotals {
    fn of(skips: &[Skip]) -> Self {
        let mut t = SkipTotals::default();
        for s in skips {
            match s.kind {
                SkipKind::Table => t.tables += s.count,
                SkipKind::Image => t.images += s.count,
                SkipKind::Footnote => t.footnotes += s.count,
            }
        }
        t
    }

    fn add(&mut self, other: &SkipTotals) {
        self.tables += other.tables;
        self.images += other.images;
        self.footnotes += other.footnotes;
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterInfo {
    /// Place in the list; what `commit` is given back.
    pub index: usize,
    /// Index of the file it came from.
    pub file: usize,
    pub title: String,
    /// Characters with spaces.
    pub chars: u32,
    pub paragraphs: usize,
    /// The beginning of the text.
    pub snippet: String,
    pub skipped: SkipTotals,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub files: Vec<FileInfo>,
    pub chapters: Vec<ChapterInfo>,
    pub skipped: SkipTotals,
}

pub fn preview(paths: &[PathBuf], opts: &ImportOptions) -> Preview {
    let (files, chapters) = read_files(paths, opts);
    let mut skipped = SkipTotals::default();
    let chapters = chapters
        .iter()
        .enumerate()
        .map(|(index, c)| {
            let totals = SkipTotals::of(&c.skips);
            skipped.add(&totals);
            let text = plain_text(&c.body);
            ChapterInfo {
                index,
                file: c.file,
                title: c.title.clone(),
                chars: count_blocks(&c.body).with_spaces,
                paragraphs: c.body.len(),
                snippet: text
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .chars()
                    .take(80)
                    .collect(),
                skipped: totals,
            }
        })
        .collect();
    Preview {
        files,
        chapters,
        skipped,
    }
}

// ---------------------------------------------------------------------------
// Making the chapters

#[derive(Debug, Clone, Deserialize)]
pub struct Pick {
    /// `ChapterInfo::index`.
    pub index: usize,
    /// The title, as edited on screen.
    pub title: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitSpec {
    /// Part to put the chapters in; the last part when missing.
    #[serde(default)]
    pub part_id: Option<String>,
    /// Put them right after this chapter; at the end when missing.
    #[serde(default)]
    pub after: Option<String>,
    pub picks: Vec<Pick>,
    /// Leave a 메모 where something was left out.
    #[serde(default)]
    pub leave_notes: bool,
    #[serde(default)]
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Committed {
    pub docs: Vec<String>,
    pub notes: usize,
}

/// Wording of a 메모 for what was left out near a paragraph.
fn skip_note(totals: &SkipTotals, near: bool) -> String {
    let mut parts = Vec::new();
    for (n, kind) in [
        (totals.tables, SkipKind::Table),
        (totals.images, SkipKind::Image),
        (totals.footnotes, SkipKind::Footnote),
    ] {
        if n > 0 {
            parts.push(format!("{} {n}개", kind.label()));
        }
    }
    let list = parts.join(" · ");
    if near {
        format!("가져오기: 이 근처에 있던 {list}은(는) 지원하지 않아 빼고 가져왔습니다.")
    } else {
        format!("가져오기: {list}은(는) 지원하지 않아 빼고 가져왔습니다.")
    }
}

fn has_text(block: &Block) -> bool {
    matches!(block, Block::Paragraph { content, .. }
        if content.iter().any(|i| matches!(i, Inline::Text { text, .. } if !text.trim().is_empty())))
}

/// Marks the text of paragraph `at` with a note. Returns the quote.
fn mark_paragraph(body: &mut [Block], at: usize, id: &str) -> String {
    let mut quote = String::new();
    if let Block::Paragraph { content, .. } = &mut body[at] {
        for inline in content {
            if let Inline::Text { text, marks } = inline {
                quote.push_str(text);
                marks.push(Mark::Memo {
                    attrs: MemoAttrs { id: id.into() },
                });
                marks.sort();
            }
        }
    }
    quote
}

/// Marks paragraphs near what was left out and returns the notes to make.
/// A place with no paragraph near becomes a note on the chapter.
fn leave_notes(body: &mut [Block], skips: &[Skip]) -> Vec<NewNote> {
    // Paragraph to mark -> what was left out there.
    let mut spots: Vec<(Option<usize>, SkipTotals)> = Vec::new();
    for s in skips {
        let before = (0..s.at.min(body.len()))
            .rev()
            .find(|&i| has_text(&body[i]));
        let spot = before.or_else(|| (s.at..body.len()).find(|&i| has_text(&body[i])));
        let one = SkipTotals::of(std::slice::from_ref(s));
        match spots.iter_mut().find(|(p, _)| *p == spot) {
            Some((_, totals)) => totals.add(&one),
            None => spots.push((spot, one)),
        }
    }
    spots
        .into_iter()
        .map(|(spot, totals)| match spot {
            Some(at) => {
                let id = new_id();
                let quote = mark_paragraph(body, at, &id);
                NewNote {
                    id: Some(id),
                    anchor: Anchor::Text,
                    target: String::new(),
                    quote,
                    text: skip_note(&totals, true),
                }
            }
            None => NewNote {
                id: None,
                anchor: Anchor::Doc,
                target: String::new(),
                quote: String::new(),
                text: skip_note(&totals, false),
            },
        })
        .collect()
}

/// Makes the chapters `spec.picks` name, in the order of the list.
pub fn commit(
    root: &Path,
    paths: &[PathBuf],
    opts: &ImportOptions,
    spec: &CommitSpec,
) -> Result<Committed> {
    if spec.picks.is_empty() {
        return Err(Error::Invalid("가져올 회차를 골라 주세요".into()));
    }
    let status = spec
        .status
        .as_deref()
        .filter(|s| STATUSES.contains(s))
        .unwrap_or("draft");
    let (_, chapters) = read_files(paths, opts);
    let mut picks = spec.picks.clone();
    picks.sort_by_key(|p| p.index);
    picks.dedup_by_key(|p| p.index);
    if picks.iter().any(|p| p.index >= chapters.len()) {
        return Err(Error::Invalid(
            "파일이 바뀌어 회차 목록이 달라짐 · 처음부터 다시 해 주세요".into(),
        ));
    }

    let mut project = project::load(root)?;
    let dir = root.join(MANUSCRIPT_DIR);
    let mut made: Vec<String> = Vec::new();
    let mut note_ids: Vec<String> = Vec::new();
    let result = (|| -> Result<()> {
        for pick in &picks {
            let chapter = &chapters[pick.index];
            let id = loop {
                let id = new_id();
                if !dir.join(doc::file_name(&id)).exists() {
                    break id;
                }
            };
            let mut body = chapter.body.clone();
            let mut new_notes = if spec.leave_notes && !chapter.skips.is_empty() {
                leave_notes(&mut body, &chapter.skips)
            } else {
                Vec::new()
            };
            let title = pick.title.trim();
            let mut meta = DocMeta::new(&id, title);
            meta.status = status.into();
            doc::write_doc_file(&dir.join(doc::file_name(&id)), &DocFile { meta, body })?;
            made.push(id.clone());
            for note in &mut new_notes {
                note.target = id.clone();
                let saved = notes::create(root, note)?;
                note_ids.push(saved.id);
            }
        }
        project::place_docs(
            &mut project,
            &made,
            spec.part_id.as_deref(),
            spec.after.as_deref(),
        );
        project::save(root, &project)
    })();
    if let Err(e) = result {
        for id in &made {
            let _ = fs::remove_file(dir.join(doc::file_name(id)));
        }
        for id in &note_ids {
            if let Ok(path) = notes::path(root, id) {
                let _ = fs::remove_file(path);
            }
        }
        return Err(e);
    }
    Ok(Committed {
        docs: made,
        notes: note_ids.len(),
    })
}
