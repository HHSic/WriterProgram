//! Counts shown to writers: characters with and without spaces, 200-cell
//! manuscript paper (원고지) lines and sheets, and what a serial platform's
//! own counter would show (연재 플랫폼 기준, docs/platforms.md).
//!
//! `app/src/editor/counts.ts` implements the same rules for the live status
//! bar; `tests/fixtures/counts.json` and `tests/fixtures/platform-counts.json`
//! keep the two in step.

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
    /// Straight marks some platforms leave out: `. , ! ? ' "` (노벨피아).
    #[serde(default)]
    pub plain_marks: u32,
    /// Characters outside the Basic Multilingual Plane (most emoji). A counter
    /// that uses JavaScript string length counts each of them twice.
    #[serde(default)]
    pub wide: u32,
    /// What `<`, `>` and `&` add when a counter reads the editor's HTML, where
    /// they are `&lt;`, `&gt;` (3 more each) and `&amp;` (4 more).
    #[serde(default)]
    pub html_extra: u32,
}

impl Counts {
    pub fn add(&mut self, other: &Counts) {
        self.with_spaces += other.with_spaces;
        self.without_spaces += other.without_spaces;
        self.manuscript_lines += other.manuscript_lines;
        self.manuscript_pages += other.manuscript_pages;
        self.plain_marks += other.plain_marks;
        self.wide += other.wide;
        self.html_extra += other.html_extra;
    }

    /// Characters as a platform with `rule` counts them. Line breaks never count.
    pub fn by_rule(&self, rule: &CountRule) -> u32 {
        let mut n = if rule.spaces {
            self.with_spaces
        } else {
            self.without_spaces
        };
        if rule.skip_marks {
            n = n.saturating_sub(self.plain_marks);
        }
        if rule.wide_twice {
            n += self.wide;
        }
        if rule.html_escapes {
            n += self.html_extra;
        }
        n
    }
}

/// How a serial platform counts characters for its minimums. The presets for
/// each platform live in `app/src/lib/platforms.ts`; a project keeps its own
/// copy so the writer can change it (docs/platforms.md).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CountRule {
    /// Spaces count (공백 포함).
    #[serde(default)]
    pub spaces: bool,
    /// `. , ! ? ' "` do not count.
    #[serde(default)]
    pub skip_marks: bool,
    /// Characters outside the Basic Multilingual Plane count as two.
    #[serde(default)]
    pub wide_twice: bool,
    /// `<`, `>` count as four and `&` as five, as in the editor's HTML.
    #[serde(default)]
    pub html_escapes: bool,
}

/// Straight marks 노벨피아 leaves out of its count.
pub fn is_plain_mark(c: char) -> bool {
    matches!(c, '.' | ',' | '!' | '?' | '\'' | '"')
}

/// What a character adds when written as HTML text.
fn html_extra(c: char) -> u32 {
    match c {
        '<' | '>' => 3,
        '&' => 4,
        _ => 0,
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
                        if is_plain_mark(c) {
                            counts.plain_marks += 1;
                        }
                        if u32::from(c) > 0xFFFF {
                            counts.wide += 1;
                        }
                        counts.html_extra += html_extra(c);
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
    fn platform_rules() {
        let c = count("“그래.” 그가 말했다! 😀 <a&b>");
        let rule = |spaces, skip_marks, wide_twice, html_escapes| CountRule {
            spaces,
            skip_marks,
            wide_twice,
            html_escapes,
        };
        assert_eq!(c.by_rule(&rule(true, false, false, false)), c.with_spaces);
        assert_eq!(
            c.by_rule(&rule(false, false, false, false)),
            c.without_spaces
        );
        // Curly quotes still count; the period and the exclamation mark do not.
        assert_eq!(c.plain_marks, 2);
        assert_eq!(
            c.by_rule(&rule(false, true, false, false)),
            c.without_spaces - 2
        );
        assert_eq!(
            c.by_rule(&rule(false, false, true, false)),
            c.without_spaces + 1
        );
        assert_eq!(
            c.by_rule(&rule(false, false, false, true)),
            c.without_spaces + 10
        );
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
