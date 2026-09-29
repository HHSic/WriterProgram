//! Page estimate (예상 쪽수) for a manuscript format.
//!
//! A rough line-breaking model, not a typesetter: Hangul and CJK take one em,
//! Latin letters and digits a little over half, spaces a third. Lines break at
//! spaces (어절 단위), as 한글 and Word are set up to do in exported files.
//! Good enough to answer "about how many pages is this in 신국판?".

use crate::format::{IndentRules, ManuscriptFormat};
use crate::markup::Block;

const MM_PER_PT: f64 = 25.4 / 72.0;

/// Lines taken by a chapter title with the space around it, in body lines.
const TITLE_LINES_NEW_PAGE: u32 = 5;
const TITLE_LINES: u32 = 3;

/// Width of a character in ems before letter spacing.
fn em_width(c: char) -> f64 {
    match c {
        ' ' | '\u{A0}' => 0.33,
        '\u{3000}' => 1.0,
        '\u{2002}' => 0.5,
        '0'..='9' | 'a'..='z' | 'A'..='Z' => 0.56,
        '.' | ',' | ':' | ';' | '!' | '?' | '\'' | '"' | '(' | ')' | '[' | ']' | '-' => 0.33,
        '“' | '”' | '‘' | '’' => 0.4,
        c if c.is_ascii() => 0.5,
        _ => 1.0,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PageMetrics {
    /// Text width in ems.
    width: f64,
    /// Extra advance per character in ems (letter spacing).
    tracking: f64,
    indent: f64,
    rules: IndentRules,
    lines_per_page: u32,
    blank_line_between: bool,
    chapter_new_page: bool,
}

impl PageMetrics {
    /// `None` for continuous formats (no paper).
    pub fn of(format: &ManuscriptFormat) -> Option<Self> {
        if !format.has_paper() {
            return None;
        }
        let m = &format.margins;
        let size_mm = format.size_pt * MM_PER_PT;
        let text_w = format.paper.width_mm - m.inside - m.outside;
        let text_h = format.paper.height_mm - m.top - m.bottom - m.header - m.footer;
        let pitch = size_mm * f64::from(format.line_spacing) / 100.0;
        if size_mm <= 0.0 || pitch <= 0.0 || text_w <= size_mm || text_h <= size_mm {
            return None;
        }
        // The first line needs only its own height; each further line one pitch.
        let lines = ((text_h - size_mm) / pitch).floor() as u32 + 1;
        Some(PageMetrics {
            width: text_w / size_mm,
            tracking: f64::from(format.letter_spacing) / 100.0,
            indent: format.indent,
            rules: format.indent_rules,
            lines_per_page: lines.max(1),
            blank_line_between: format.blank_line_between,
            chapter_new_page: format.chapter_new_page,
        })
    }

    pub fn lines_per_page(&self) -> u32 {
        self.lines_per_page
    }

    fn advance(&self, c: char) -> f64 {
        em_width(c) + self.tracking
    }

    /// Lines one line of text (between line breaks) takes.
    #[cfg(test)]
    fn wrap(&self, text: &str, indent: f64) -> u32 {
        self.wrap_in(text, indent, self.width)
    }

    /// The same within a narrower column (문단 여백).
    fn wrap_in(&self, text: &str, indent: f64, width: f64) -> u32 {
        let mut lines = 1;
        let mut x = indent;
        let mut word = 0.0;
        let mut word_chars: Vec<f64> = Vec::new();
        let place_word = |x: &mut f64, lines: &mut u32, word: f64, chars: &[f64]| {
            if word == 0.0 {
                return;
            }
            if *x + word <= width + 1e-9 {
                *x += word;
            } else if word <= width {
                *lines += 1;
                *x = word;
            } else {
                // Longer than a line: break between characters.
                for &w in chars {
                    if *x + w > width + 1e-9 {
                        *lines += 1;
                        *x = 0.0;
                    }
                    *x += w;
                }
            }
        };
        for c in text.chars() {
            if c == ' ' || c == '\u{3000}' || c == '\u{2002}' {
                place_word(&mut x, &mut lines, word, &word_chars);
                word = 0.0;
                word_chars.clear();
                // A space that does not fit ends the line and is not carried over.
                let w = self.advance(c);
                if x + w <= width + 1e-9 {
                    x += w;
                }
            } else {
                let w = self.advance(c);
                word += w;
                word_chars.push(w);
            }
        }
        place_word(&mut x, &mut lines, word, &word_chars);
        lines
    }

    /// Body lines of one chapter, without its title.
    pub fn body_lines(&self, blocks: &[Block]) -> u32 {
        let mut lines = 0;
        for block in &crate::indent::apply(blocks, self.indent, self.rules) {
            match block {
                Block::SceneBreak {} => lines += if self.blank_line_between { 1 } else { 3 },
                Block::Paragraph { attrs, .. } => {
                    // Margins are whole characters, letter spacing included.
                    let sides = f64::from(attrs.left) + f64::from(attrs.right);
                    let width = (self.width - sides * (1.0 + self.tracking)).max(1.0);
                    for (i, line) in block.lines().iter().enumerate() {
                        let first = attrs.indent.map_or(self.indent, f64::from);
                        lines += self.wrap_in(line, if i == 0 { first } else { 0.0 }, width);
                    }
                }
            }
            if self.blank_line_between {
                lines += 1;
            }
        }
        lines
    }

    /// Pages of one chapter when it starts on a new page.
    pub fn chapter_pages(&self, blocks: &[Block], with_title: bool) -> u32 {
        let title = if with_title { TITLE_LINES_NEW_PAGE } else { 0 };
        (self.body_lines(blocks) + title)
            .div_ceil(self.lines_per_page)
            .max(1)
    }

    /// Pages of several chapters.
    pub fn pages<'a>(
        &self,
        chapters: impl IntoIterator<Item = &'a [Block]>,
        with_titles: bool,
    ) -> u32 {
        if self.chapter_new_page {
            chapters
                .into_iter()
                .map(|blocks| self.chapter_pages(blocks, with_titles))
                .sum()
        } else {
            let title = if with_titles { TITLE_LINES } else { 0 };
            let lines: u32 = chapters
                .into_iter()
                .map(|blocks| self.body_lines(blocks) + title)
                .sum();
            lines.div_ceil(self.lines_per_page).max(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::format::{builtin, default_for};
    use crate::project::ProjectKind;

    fn para(n: usize) -> Block {
        Block::text(&"가".repeat(n))
    }

    #[test]
    fn continuous_formats_have_no_pages() {
        assert!(PageMetrics::of(&default_for(ProjectKind::Webnovel)).is_none());
    }

    #[test]
    fn a4_submission_page_holds_about_forty_lines_of_forty_chars() {
        let m = PageMetrics::of(&builtin("submission-a4").unwrap()).unwrap();
        // 297 - 20 - 15 - 15 - 15 = 232mm of text, 10pt at 160% = 5.64mm a line.
        assert_eq!(m.lines_per_page(), 41);
        // 150mm / 3.53mm = 42.5 characters a line; the first line is indented.
        assert_eq!(m.wrap(&"가".repeat(41), 1.0), 1);
        assert_eq!(m.wrap(&"가".repeat(42), 1.0), 2);
        assert_eq!(m.wrap(&"가".repeat(42), 0.0), 1);
    }

    #[test]
    fn paragraph_margins_narrow_the_line() {
        use crate::markup::{Inline, ParaAttrs};
        let m = PageMetrics::of(&builtin("submission-a4").unwrap()).unwrap();
        let text = "가".repeat(40);
        let plain = Block::text(&text);
        let letter = Block::Paragraph {
            attrs: ParaAttrs {
                left: 4,
                right: 2,
                indent: None,
            },
            content: vec![Inline::Text {
                text: text.clone(),
                marks: vec![],
            }],
        };
        assert_eq!(m.body_lines(&[plain]), 1);
        assert_eq!(m.body_lines(&[letter]), 2);
    }

    #[test]
    fn words_move_to_the_next_line_whole() {
        let m = PageMetrics::of(&builtin("submission-a4").unwrap()).unwrap();
        let fits = "가".repeat(40);
        // A three-character word that does not fit goes down whole.
        assert_eq!(m.wrap(&format!("{fits} 나나나"), 0.0), 2);
        // A very long word breaks between characters.
        assert_eq!(m.wrap(&"가".repeat(100), 0.0), 3);
    }

    #[test]
    fn chapters_start_on_new_pages() {
        let m = PageMetrics::of(&builtin("submission-a4").unwrap()).unwrap();
        let short = vec![para(10)];
        assert_eq!(m.pages([short.as_slice(), short.as_slice()], true), 2);
        let long: Vec<Block> = (0..80).map(|_| para(10)).collect();
        assert_eq!(m.chapter_pages(&long, true), 3);
    }

    #[test]
    fn a_book_page_holds_less_than_a_manuscript_page() {
        let text: Vec<Block> = (0..400).map(|_| para(120)).collect();
        let a4 = PageMetrics::of(&builtin("submission-a4").unwrap()).unwrap();
        let book = PageMetrics::of(&builtin("book-shinguk").unwrap()).unwrap();
        let (a4_pages, book_pages) = (
            a4.chapter_pages(&text, false),
            book.chapter_pages(&text, false),
        );
        assert!(book_pages > a4_pages * 3 / 2, "{a4_pages} vs {book_pages}");
    }
}
