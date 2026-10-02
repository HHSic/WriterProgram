//! A paragraph being read from a docx or HWPX file, and how it ends up as
//! an [`Item`].

use super::text::is_scene;
use super::{Item, SkipKind};
use crate::markup::{Inline, Mark, inline_text};

/// What a run of text looks like.
#[derive(Default, Clone, Copy)]
pub(super) struct Run {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub underline: bool,
    pub dot: bool,
}

impl Run {
    pub(super) fn marks(self) -> Vec<Mark> {
        let mut marks = Vec::new();
        if self.underline {
            marks.push(Mark::Underline {});
        }
        if self.dot {
            marks.push(Mark::Dot {});
        }
        if self.strike {
            marks.push(Mark::Strike {});
        }
        if self.bold {
            marks.push(Mark::Bold {});
        }
        if self.italic {
            marks.push(Mark::Italic {});
        }
        marks
    }
}

#[derive(Default)]
pub(super) struct Para {
    /// docx only: the paragraph style and outline level.
    pub style: Option<String>,
    pub outline: Option<u8>,
    pub inlines: Vec<Inline>,
    pub skips: Vec<(SkipKind, u32)>,
}

impl Para {
    pub(super) fn add_text(&mut self, text: &str, run: Run) {
        if text.is_empty() {
            return;
        }
        let marks = run.marks();
        if let Some(Inline::Text {
            text: last,
            marks: m,
        }) = self.inlines.last_mut()
            && *m == marks
        {
            last.push_str(text);
            return;
        }
        self.inlines.push(Inline::Text {
            text: text.into(),
            marks,
        });
    }

    pub(super) fn skip(&mut self, kind: SkipKind) {
        match self.skips.iter_mut().find(|(k, _)| *k == kind) {
            Some((_, n)) => *n += 1,
            None => self.skips.push((kind, 1)),
        }
    }

    /// Ends the paragraph. Text becomes a scene break, a heading of `level`
    /// (one line only) or a paragraph; empty paragraphs and 차례 lines
    /// (`toc`) are dropped. What was left out of it follows.
    pub(super) fn close(mut self, level: Option<u8>, toc: bool, items: &mut Vec<Item>) {
        trim_ends(&mut self.inlines);
        let text = inline_text(&self.inlines);
        if !text.trim().is_empty() && !toc {
            let single = !text.contains('\n');
            if is_scene(&text) && single && level.is_none() {
                items.push(Item::Scene);
            } else if let (Some(level), true) = (level, single) {
                items.push(Item::Heading {
                    level,
                    text: text.trim().to_string(),
                });
            } else {
                items.push(Item::Para(self.inlines));
            }
        }
        for (kind, n) in self.skips {
            items.push(Item::Skip(kind, n));
        }
    }
}

/// Takes spaces (and the full-width space some writers indent with) off both
/// ends of a paragraph, and line breaks with them.
fn trim_ends(inlines: &mut Vec<Inline>) {
    loop {
        match inlines.first_mut() {
            Some(Inline::HardBreak {}) => {
                inlines.remove(0);
            }
            Some(Inline::Text { text, .. }) => {
                let trimmed = text.trim_start();
                if trimmed.is_empty() {
                    inlines.remove(0);
                } else {
                    *text = trimmed.to_string();
                    break;
                }
            }
            None => return,
        }
    }
    loop {
        match inlines.last_mut() {
            Some(Inline::HardBreak {}) => {
                inlines.pop();
            }
            Some(Inline::Text { text, .. }) => {
                let trimmed = text.trim_end();
                if trimmed.is_empty() {
                    inlines.pop();
                } else {
                    *text = trimmed.to_string();
                    break;
                }
            }
            None => return,
        }
    }
}
