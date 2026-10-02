//! 가져오기: txt, md, docx and hwpx files become chapters.
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

mod hangul;
mod notes;
mod page;
mod para;
mod split;
mod text;
mod word;
mod xml;

pub use page::PageSetup;

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use notes::leave_notes;
use split::{resolve, split};

use crate::count::count_blocks;
use crate::doc::{self, DocFile, DocMeta, STATUSES};
use crate::markup::{Block, Inline, plain_text};
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
    /// hwpx only.
    pub page: Option<PageSetup>,
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
        "hwpx" => hangul::read(&read()?),
        "hwp" => Err("옛 한글 형식(.hwp)은 읽을 수 없음 · 한글에서 hwpx로 저장해 주세요".into()),
        _ => Err("가져올 수 없는 파일 형식 (txt, md, docx, hwpx)".into()),
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
    /// Paper, margins, 머리말, 꼬리말 and page numbers (hwpx).
    pub page: Option<PageSetup>,
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
            page: None,
        };
        let result = read_raw(path, opts).and_then(|raw| {
            let how = resolve(&raw.items, opts)?;
            info.encoding = raw.encoding;
            info.page = raw.page;
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
    /// Make the 원고 서식 follow the pages of the first 한글 file.
    #[serde(default)]
    pub page_setup: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Committed {
    pub docs: Vec<String>,
    pub notes: usize,
    /// The 원고 서식 now follows the file's pages.
    pub format: bool,
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
    let (files, chapters) = read_files(paths, opts);
    let mut picks = spec.picks.clone();
    picks.sort_by_key(|p| p.index);
    picks.dedup_by_key(|p| p.index);
    if picks.iter().any(|p| p.index >= chapters.len()) {
        return Err(Error::Invalid(
            "파일이 바뀌어 회차 목록이 달라짐 · 처음부터 다시 해 주세요".into(),
        ));
    }

    let mut project = project::load(root)?;
    // The format is checked before anything is made.
    if spec.page_setup {
        let setup = files
            .iter()
            .find_map(|f| f.page.as_ref())
            .ok_or_else(|| Error::Invalid("원고 서식에 맞출 쪽 모양이 파일에 없음".into()))?;
        let mut format = project.manuscript_format();
        setup.apply(&mut format);
        format.validate()?;
        project.manuscript_format = Some(format);
    }
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
                let saved = crate::notes::create(root, note)?;
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
            if let Ok(path) = crate::notes::path(root, id) {
                let _ = fs::remove_file(path);
            }
        }
        return Err(e);
    }
    Ok(Committed {
        docs: made,
        notes: note_ids.len(),
        format: spec.page_setup,
    })
}
