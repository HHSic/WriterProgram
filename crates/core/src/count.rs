//! Counts shown to writers: characters with and without spaces, and 200-cell
//! manuscript paper (원고지) lines and sheets.
//!
//! `app/src/editor/counts.ts` implements the same rules for the live status
//! bar; `tests/fixtures/counts.json` keeps the two in step.

use serde::{Deserialize, Serialize};

use crate::markup::Block;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Counts {
    pub with_spaces: u32,
    pub without_spaces: u32,
    /// Lines on 원고지 (20 cells a line, 10 lines a sheet).
    pub manuscript_lines: u32,
    /// Sheets of 200-cell 원고지, rounded up. A new chapter starts on a new sheet,
    /// so totals add sheets rather than lines.
    pub manuscript_pages: u32,
}

impl Counts {
    pub fn add(&mut self, other: &Counts) {
        self.with_spaces += other.with_spaces;
        self.without_spaces += other.without_spaces;
        self.manuscript_lines += other.manuscript_lines;
        self.manuscript_pages += other.manuscript_pages;
    }
}

/// Whitespace for "공백 제외". An explicit list so the TypeScript side can match
/// it exactly.
pub fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n'
            | '\u{0B}'
            | '\u{0C}'
            | '\r'
            | ' '
            | '\u{85}'
            | '\u{A0}'
            | '\u{1680}'
            | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

pub fn count_blocks(blocks: &[Block]) -> Counts {
    let mut counts = Counts::default();
    let mut lines = 0;
    for block in blocks {
        match block {
            Block::SceneBreak {} => lines += 1,
            Block::Paragraph { .. } => {
                for (i, line) in block.lines().iter().enumerate() {
                    for c in line.chars() {
                        counts.with_spaces += 1;
                        if !is_space(c) {
                            counts.without_spaces += 1;
                        }
                    }
                    lines += paper_lines(line, i == 0);
                }
            }
        }
    }
    if counts.with_spaces > 0 {
        counts.manuscript_lines = lines;
        counts.manuscript_pages = lines.div_ceil(10);
    }
    counts
}

/// Half cells in one 원고지 line (20 cells).
const LINE_HALF_CELLS: u32 = 40;

/// Arabic numerals and lowercase Latin letters take half a cell; everything
/// else takes a whole cell.
fn half_cells(c: char) -> u32 {
    if c.is_ascii_digit() || c.is_ascii_lowercase() {
        1
    } else {
        2
    }
}

/// Punctuation that does not start a line: when the line is full it is written
/// in the margin of the line it closes.
fn hangs(c: char) -> bool {
    matches!(
        c,
        '.' | ',' | '!' | '?' | ':' | ';' | ')' | ']' | '}' | '”' | '’' | '」' | '』' | '》' | '〉'
    )
}

/// Lines one line of text takes on 원고지. The first line of a paragraph starts
/// with one empty cell. No blank cell after a period or comma, and a space that
/// falls at the start of a line is not written.
fn paper_lines(text: &str, indent: bool) -> u32 {
    let mut lines = 1;
    let mut col = if indent { 2 } else { 0 };
    let mut prev: Option<char> = None;
    for c in text.chars() {
        let space = is_space(c);
        if space && matches!(prev, Some('.') | Some(',')) {
            prev = Some(c);
            continue;
        }
        let width = if space { 2 } else { half_cells(c) };
        if col + width > LINE_HALF_CELLS {
            if hangs(c) {
                prev = Some(c);
                continue;
            }
            lines += 1;
            col = 0;
            if space {
                prev = Some(c);
                continue;
            }
        }
        col += width;
        prev = Some(c);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count(text: &str) -> Counts {
        let blocks: Vec<Block> = text.split("\n\n").map(Block::text).collect();
        count_blocks(&blocks)
    }

    #[test]
    fn characters() {
        let c = count("가나 다\u{3000}라");
        assert_eq!((c.with_spaces, c.without_spaces), (6, 4));
        // A line break is not a character.
        let c = count("첫 줄\n둘째 줄");
        assert_eq!((c.with_spaces, c.without_spaces), (7, 5));
    }

    #[test]
    fn empty_body_is_zero_sheets() {
        assert_eq!(count_blocks(&[Block::text("")]), Counts::default());
    }

    #[test]
    fn paper_lines_follow_manuscript_rules() {
        // 19 characters fit after the indent; the 20th wraps.
        assert_eq!(paper_lines(&"가".repeat(19), true), 1);
        assert_eq!(paper_lines(&"가".repeat(20), true), 2);
        // Without indent (after a line break) 20 fit.
        assert_eq!(paper_lines(&"가".repeat(20), false), 1);
        // Two digits share a cell.
        assert_eq!(paper_lines(&"1".repeat(38), true), 1);
        // A period at the start of a new line hangs on the previous one.
        assert_eq!(paper_lines(&format!("{}.", "가".repeat(19)), true), 1);
        // No blank after a period or comma.
        assert_eq!(
            paper_lines("했다. 그리고", true),
            paper_lines("했다.그리고", true)
        );
        // A space that would start a line is dropped.
        assert_eq!(paper_lines(&format!("{} 나", "가".repeat(19)), true), 2);
    }

    #[test]
    fn sheets_round_up() {
        let para = "가".repeat(38); // 2 lines each
        let text = vec![para; 6].join("\n\n"); // 12 lines
        let c = count(&text);
        assert_eq!((c.manuscript_lines, c.manuscript_pages), (12, 2));
    }
}
