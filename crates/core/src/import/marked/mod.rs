//! Reading a corrected file (교정본) the way 교정본 주고받기 needs it: every
//! character with what it looks like (underline, strikethrough, colour,
//! highlight) and whether it is a tracked insertion or deletion (한글's
//! 변경 내용 추적, Word's track changes), plus the editor's notes (한글 메모,
//! Word comments) with the text they are on.
//!
//! The text comes out as one stream of [`Cell`]s in which every paragraph
//! ends with [`PARA`], a scene break is the single character [`SCENE`] and a
//! line break inside a paragraph is `\n`. Headings (chapter titles, which
//! the app keeps outside the text) and empty paragraphs are left out.
//! Tables, pictures and other objects are skipped as in a normal import.
//!
//! Normal import (`super::read_raw`) does not use any of this.

mod hangul;
mod word;

use std::fs;
use std::path::Path;

use super::para::Run;
use super::text::is_scene;
use crate::Error;

/// Ends every paragraph in a stream (U+2029 PARAGRAPH SEPARATOR).
pub(crate) const PARA: char = '\u{2029}';
/// Stands for a scene break, which is a paragraph of its own (private use).
pub(crate) const SCENE: char = '\u{E000}';

/// What a character looks like, as far as corrections go.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct Look {
    pub underline: bool,
    pub strike: bool,
    /// Coloured text (anything but black).
    pub color: bool,
    /// Highlighter (형광펜) or shading behind the text.
    pub shade: bool,
}

impl Look {
    fn of(run: Run) -> Look {
        Look {
            underline: run.underline,
            strike: run.strike,
            color: run.color,
            shade: run.shade,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Edit {
    Insert,
    Delete,
}

/// A tracked insertion or deletion; `by` indexes [`Marked::who`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Tracked {
    pub edit: Edit,
    pub by: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cell {
    pub ch: char,
    pub look: Look,
    pub tracked: Option<Tracked>,
}

/// Who made a tracked change, and when (as the file gives it).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Who {
    pub author: String,
    pub date: String,
}

/// A note the editor left in the file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FileNote {
    pub text: String,
    pub author: String,
    pub date: String,
    /// Cells it is on, `start..end`; none when the file does not say.
    pub on: Option<(usize, usize)>,
}

#[derive(Debug, Default)]
pub(crate) struct Marked {
    pub cells: Vec<Cell>,
    pub who: Vec<Who>,
    pub notes: Vec<FileNote>,
}

/// Reads a corrected 한글 (hwpx) or Word (docx) file. `scene_mark` is the
/// scene break symbol the file was sent with.
pub(crate) fn read(path: &Path, scene_mark: &str) -> Result<Marked, String> {
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "hwpx" | "docx" => {
            let bytes = fs::read(path).map_err(|e| Error::io(path, e).user_message())?;
            read_bytes(&bytes, &ext, scene_mark)
        }
        "hwp" => Err("옛 한글 형식(.hwp)은 읽을 수 없음 · 한글에서 hwpx로 저장해 주세요".into()),
        "doc" => Err("옛 Word 형식(.doc)은 읽을 수 없음 · Word에서 docx로 저장해 주세요".into()),
        _ => Err("교정본은 한글(hwpx)이나 Word(docx) 파일이어야 함".into()),
    }
}

/// [`read`] of a file's bytes; `ext` is `hwpx` or `docx`.
pub(crate) fn read_bytes(bytes: &[u8], ext: &str, scene_mark: &str) -> Result<Marked, String> {
    if ext == "docx" {
        word::read(bytes, scene_mark)
    } else {
        hangul::read(bytes, scene_mark)
    }
}

/// Builds the cell stream paragraph by paragraph.
#[derive(Default)]
struct Builder {
    out: Marked,
    /// Where the open paragraph's cells begin.
    start: usize,
    scene_mark: String,
}

impl Builder {
    fn new(scene_mark: &str) -> Builder {
        Builder {
            scene_mark: scene_mark.trim().to_string(),
            ..Builder::default()
        }
    }

    fn begin(&mut self) {
        self.start = self.out.cells.len();
    }

    fn push(&mut self, text: &str, look: Look, tracked: Option<Tracked>) {
        self.out
            .cells
            .extend(text.chars().map(|ch| Cell { ch, look, tracked }));
    }

    /// Ends the open paragraph. A heading or an empty paragraph is dropped;
    /// a paragraph that reads as a scene break becomes [`SCENE`]. `tracked`
    /// says whether the paragraph's end itself was inserted (the paragraph
    /// was split) or deleted (joined with the next).
    fn end(&mut self, heading: bool, tracked: Option<Tracked>) {
        let cells = &mut self.out.cells;
        if heading || cells.len() == self.start {
            cells.truncate(self.start);
            return;
        }
        let kept: String = cells[self.start..]
            .iter()
            .filter(|c| !matches!(c.tracked, Some(t) if t.edit == Edit::Delete))
            .map(|c| c.ch)
            .collect();
        let untracked = cells[self.start..].iter().all(|c| c.tracked.is_none());
        let scene =
            is_scene(&kept) || (!self.scene_mark.is_empty() && kept.trim() == self.scene_mark);
        if untracked && !kept.contains('\n') && scene {
            cells.truncate(self.start);
            cells.push(Cell {
                ch: SCENE,
                look: Look::default(),
                tracked: None,
            });
        }
        cells.push(Cell {
            ch: PARA,
            look: Look::default(),
            tracked,
        });
    }

    fn finish(mut self) -> Marked {
        let len = self.out.cells.len();
        for note in &mut self.out.notes {
            if let Some((s, e)) = note.on.as_mut() {
                *s = (*s).min(len);
                *e = (*e).clamp(*s, len);
            }
        }
        self.out
    }
}
