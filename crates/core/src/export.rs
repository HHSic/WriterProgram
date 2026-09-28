//! Export: plain text (txt file or clipboard), Word (docx) and 한글 (HWPX).

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::format::ManuscriptFormat;
use crate::markup::{Block, Inline, Mark};
use crate::store::{atomic_write, safe_file_name};
use crate::{Error, Result, doc, docx, hwpx, project};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextOptions {
    /// Put each document's heading line above its text.
    pub include_titles: bool,
    /// Separate paragraphs with an empty line (web novel platforms) instead of
    /// a single line break (print manuscripts).
    pub blank_line_between: bool,
    /// Symbol for scene breaks, e.g. "◆" or "* * *".
    pub scene_break: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportItem {
    pub doc_id: String,
    /// Heading line, e.g. "12화 비에 젖은 손님". The UI builds it.
    pub heading: String,
    /// File name without extension when exporting one file per document.
    #[serde(default)]
    pub file_name: String,
}

pub fn body_text(blocks: &[Block], opts: &TextOptions) -> String {
    let sep = if opts.blank_line_between {
        "\n\n"
    } else {
        "\n"
    };
    let mut out = String::new();
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            out.push_str(sep);
        }
        match block {
            Block::SceneBreak {} => {
                if !opts.blank_line_between {
                    out.push('\n');
                }
                out.push_str(&opts.scene_break);
                if !opts.blank_line_between {
                    out.push('\n');
                }
            }
            Block::Paragraph { attrs, .. } => {
                // 문단 여백 on the left as full-width spaces, the way plain text shows it.
                let lead = "\u{3000}".repeat(usize::from(attrs.left));
                let lines: Vec<String> =
                    block.lines().iter().map(|l| format!("{lead}{l}")).collect();
                out.push_str(&lines.join("\n"));
            }
        }
    }
    out
}

fn item_text(root: &Path, item: &ExportItem, opts: &TextOptions) -> Result<String> {
    let file = doc::load(root, &item.doc_id)?;
    let body = body_text(&file.body, opts);
    Ok(if opts.include_titles && !item.heading.trim().is_empty() {
        format!("{}\n\n{body}", item.heading.trim())
    } else {
        body
    })
}

/// All items as one text with `\n` line ends, documents separated by two
/// empty lines. Used for the clipboard and single-file export.
pub fn items_text(root: &Path, items: &[ExportItem], opts: &TextOptions) -> Result<String> {
    let parts = items
        .iter()
        .map(|item| item_text(root, item, opts))
        .collect::<Result<Vec<_>>>()?;
    Ok(parts.join("\n\n\n"))
}

/// Writes text as a `.txt` file that Notepad and 한글 open without broken
/// characters: UTF-8 with a byte order mark and CRLF line ends.
pub fn write_txt(path: &Path, text: &str) -> Result<()> {
    let mut bytes = Vec::with_capacity(text.len() + 64);
    bytes.extend_from_slice(b"\xEF\xBB\xBF");
    bytes.extend_from_slice(text.replace('\n', "\r\n").as_bytes());
    bytes.extend_from_slice(b"\r\n");
    atomic_write(path, &bytes)
}

/// Exports to one file at `dest`, or with `per_doc` one file per item inside
/// the folder `dest`. Returns the files written.
pub fn export_txt(
    root: &Path,
    items: &[ExportItem],
    opts: &TextOptions,
    dest: &Path,
    per_doc: bool,
) -> Result<Vec<PathBuf>> {
    if items.is_empty() {
        return Err(Error::Invalid("내보낼 회차를 골라 주세요".into()));
    }
    if !per_doc {
        write_txt(dest, &items_text(root, items, opts)?)?;
        return Ok(vec![dest.to_path_buf()]);
    }
    let paths = per_doc_paths(dest, items, "txt");
    for (item, path) in items.iter().zip(&paths) {
        write_txt(path, &item_text(root, item, opts)?)?;
    }
    Ok(paths)
}

/// One file per item inside the folder `dest`, with unique names.
fn per_doc_paths(dest: &Path, items: &[ExportItem], ext: &str) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = Vec::with_capacity(items.len());
    for item in items {
        let base = safe_file_name(&item.file_name, &item.doc_id);
        let mut path = dest.join(format!("{base}.{ext}"));
        let mut n = 2;
        while paths.contains(&path) {
            path = dest.join(format!("{base} ({n}).{ext}"));
            n += 1;
        }
        paths.push(path);
    }
    paths
}

// ---------------------------------------------------------------------------
// Formatted export (docx, HWPX)

/// Marks that show on paper. Memo anchors are not printed.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct RunStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub dot: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Piece {
    Text(String, RunStyle),
    Break,
}

/// A paragraph as runs of text with one style each, and line breaks.
pub fn pieces(content: &[Inline]) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    for inline in content {
        match inline {
            Inline::HardBreak {} => out.push(Piece::Break),
            Inline::Text { text, marks } => {
                let style = RunStyle {
                    bold: marks.contains(&Mark::Bold {}),
                    italic: marks.contains(&Mark::Italic {}),
                    underline: marks.contains(&Mark::Underline {}),
                    strike: marks.contains(&Mark::Strike {}),
                    dot: marks.contains(&Mark::Dot {}),
                };
                for (i, part) in text.split('\n').enumerate() {
                    if i > 0 {
                        out.push(Piece::Break);
                    }
                    if part.is_empty() {
                        continue;
                    }
                    match out.last_mut() {
                        Some(Piece::Text(prev, prev_style)) if *prev_style == style => {
                            prev.push_str(part)
                        }
                        _ => out.push(Piece::Text(part.to_string(), style)),
                    }
                }
            }
        }
    }
    out
}

/// One chapter to export: its heading line and text.
pub struct ExportDoc {
    pub heading: String,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocOptions {
    /// Put each chapter's heading above its text.
    pub include_titles: bool,
    /// Symbol for scene breaks.
    pub scene_break: String,
}

/// Document properties written into the file.
#[derive(Debug, Clone, Default)]
pub struct DocInfo {
    pub title: String,
    pub author: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileKind {
    Docx,
    Hwpx,
}

impl FileKind {
    fn ext(self) -> &'static str {
        match self {
            FileKind::Docx => "docx",
            FileKind::Hwpx => "hwpx",
        }
    }

    fn build(
        self,
        docs: &[ExportDoc],
        format: &ManuscriptFormat,
        opts: &DocOptions,
        info: &DocInfo,
    ) -> Result<Vec<u8>> {
        match self {
            FileKind::Docx => docx::docx_bytes(docs, format, opts, info),
            FileKind::Hwpx => hwpx::hwpx_bytes(docs, format, opts, info),
        }
    }
}

/// Exports chapters as docx or HWPX with `format` applied: to one file at
/// `dest`, or with `per_doc` one file per item inside the folder `dest`.
pub fn export_file(
    root: &Path,
    items: &[ExportItem],
    opts: &DocOptions,
    format: &ManuscriptFormat,
    kind: FileKind,
    dest: &Path,
    per_doc: bool,
) -> Result<Vec<PathBuf>> {
    if items.is_empty() {
        return Err(Error::Invalid("내보낼 회차를 골라 주세요".into()));
    }
    format.validate()?;
    let project = project::load(root)?;
    let info = DocInfo {
        title: project.title.clone(),
        author: project.pen_name.clone(),
    };
    let load = |item: &ExportItem| -> Result<ExportDoc> {
        Ok(ExportDoc {
            heading: item.heading.clone(),
            blocks: doc::load(root, &item.doc_id)?.body,
        })
    };
    if !per_doc {
        let docs = items.iter().map(load).collect::<Result<Vec<_>>>()?;
        atomic_write(dest, &kind.build(&docs, format, opts, &info)?)?;
        return Ok(vec![dest.to_path_buf()]);
    }
    let paths = per_doc_paths(dest, items, kind.ext());
    for (item, path) in items.iter().zip(&paths) {
        let docs = [load(item)?];
        atomic_write(path, &kind.build(&docs, format, opts, &info)?)?;
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(blank: bool) -> TextOptions {
        TextOptions {
            include_titles: true,
            blank_line_between: blank,
            scene_break: "◆".into(),
        }
    }

    #[test]
    fn left_margin_as_full_width_spaces() {
        use crate::markup::{Inline, ParaAttrs};
        let blocks = vec![
            Block::text("그리고"),
            Block::Paragraph {
                attrs: ParaAttrs { left: 2, right: 1 },
                content: vec![
                    Inline::Text {
                        text: "첫 줄".into(),
                        marks: vec![],
                    },
                    Inline::HardBreak {},
                    Inline::Text {
                        text: "둘째 줄".into(),
                        marks: vec![],
                    },
                ],
            },
        ];
        assert_eq!(
            body_text(&blocks, &opts(false)),
            "그리고
　　첫 줄
　　둘째 줄"
        );
    }

    #[test]
    fn web_novel_and_print_layouts() {
        let blocks = vec![
            Block::text("첫 문단"),
            Block::text("둘째\n줄바꿈"),
            Block::SceneBreak {},
            Block::text("셋째"),
        ];
        assert_eq!(
            body_text(&blocks, &opts(true)),
            "첫 문단\n\n둘째\n줄바꿈\n\n◆\n\n셋째"
        );
        assert_eq!(
            body_text(&blocks, &opts(false)),
            "첫 문단\n둘째\n줄바꿈\n\n◆\n\n셋째"
        );
    }

    #[test]
    fn pieces_merge_runs_and_keep_breaks() {
        let content = vec![
            Inline::Text {
                text: "가".into(),
                marks: vec![Mark::Bold {}],
            },
            Inline::Text {
                text: "나".into(),
                marks: vec![
                    Mark::Bold {},
                    Mark::Memo {
                        attrs: crate::markup::MemoAttrs { id: "m".into() },
                    },
                ],
            },
            Inline::HardBreak {},
            Inline::Text {
                text: "다\n라".into(),
                marks: vec![],
            },
        ];
        let bold = RunStyle {
            bold: true,
            ..RunStyle::default()
        };
        assert_eq!(
            pieces(&content),
            vec![
                Piece::Text("가나".into(), bold),
                Piece::Break,
                Piece::Text("다".into(), RunStyle::default()),
                Piece::Break,
                Piece::Text("라".into(), RunStyle::default()),
            ]
        );
    }
}
