//! Plain-text export (txt file or clipboard).

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::markup::Block;
use crate::store::{atomic_write, safe_file_name};
use crate::{Error, Result, doc};

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
            Block::Paragraph { .. } => out.push_str(&block.lines().join("\n")),
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
    let mut written: Vec<PathBuf> = Vec::with_capacity(items.len());
    for item in items {
        let base = safe_file_name(&item.file_name, &item.doc_id);
        let mut path = dest.join(format!("{base}.txt"));
        let mut n = 2;
        while written.contains(&path) {
            path = dest.join(format!("{base} ({n}).txt"));
            n += 1;
        }
        write_txt(&path, &item_text(root, item, opts)?)?;
        written.push(path);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup::Block;

    fn opts(blank: bool) -> TextOptions {
        TextOptions {
            include_titles: true,
            blank_line_between: blank,
            scene_break: "◆".into(),
        }
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
}
