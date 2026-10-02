//! 메모 left where tables, pictures or footnotes were taken out.

use super::{Skip, SkipKind, SkipTotals};
use crate::markup::{Block, Inline, Mark, MemoAttrs};
use crate::notes::{Anchor, NewNote};
use crate::store::new_id;

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
pub(super) fn leave_notes(body: &mut [Block], skips: &[Skip]) -> Vec<NewNote> {
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
