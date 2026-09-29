//! First-line indents (들여쓰기) paragraph by paragraph: a paragraph's own
//! first line (문단 모양) when it has one, else the manuscript format's indent
//! with its rules (format::IndentRules). Exports and the page estimate turn
//! the rules into each paragraph's own shape first, so the rest of the code
//! only knows "the format's indent, unless the paragraph says otherwise".

use crate::format::IndentRules;
use crate::markup::{Block, Inline, ParaAttrs};

/// Marks that open a line of dialogue.
const QUOTES: [char; 8] = ['“', '"', '「', '『', '‘', '\'', '《', '«'];

/// Whether a paragraph opens with a quotation mark (spaces before it aside).
pub fn is_dialogue(content: &[Inline]) -> bool {
    for inline in content {
        match inline {
            Inline::Text { text, .. } => {
                if let Some(c) = text.chars().find(|c| !c.is_whitespace()) {
                    return QUOTES.contains(&c);
                }
            }
            Inline::HardBreak {} => return false,
        }
    }
    false
}

/// What the rules make of each paragraph of a chapter: its shape with the
/// first line (and for 원고지-style dialogue, the left margin) worked out.
/// Paragraphs the rules leave alone keep their shape; ones with their own
/// first line keep it. `indent` is the format's, in characters.
pub fn apply(blocks: &[Block], indent: f64, rules: IndentRules) -> Vec<Block> {
    let mut out = Vec::with_capacity(blocks.len());
    let mut after_scene = false;
    let mut first = true;
    for block in blocks {
        match block {
            Block::SceneBreak {} => {
                after_scene = true;
                out.push(block.clone());
                continue;
            }
            Block::Paragraph { attrs, content } => {
                let mut shaped = *attrs;
                if attrs.indent.is_none() && indent > 0.0 {
                    let flush = (rules.chapter_first && first)
                        || (rules.after_scene && after_scene)
                        || (rules.margined && (attrs.left > 0 || attrs.right > 0));
                    let dialogue = is_dialogue(content);
                    if flush || (rules.dialogue && dialogue) {
                        shaped.indent = Some(0);
                    } else if rules.dialogue_hang && dialogue {
                        // 원고지: every line starts one cell in, the first no further.
                        let cells = indent.round().clamp(0.0, f64::from(ParaAttrs::MAX)) as u8;
                        shaped.left = shaped.left.saturating_add(cells).min(ParaAttrs::MAX);
                        shaped.indent = Some(0);
                    }
                }
                out.push(Block::Paragraph {
                    attrs: shaped,
                    content: content.clone(),
                });
                // An empty paragraph between is still "right after" for the reader.
                let empty = content.is_empty();
                if !empty {
                    first = false;
                    after_scene = false;
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(text: &str) -> Block {
        Block::text(text)
    }

    fn firsts(blocks: &[Block]) -> Vec<(Option<i8>, u8)> {
        blocks
            .iter()
            .filter_map(|b| match b {
                Block::Paragraph { attrs, .. } => Some((attrs.indent, attrs.left)),
                Block::SceneBreak {} => None,
            })
            .collect()
    }

    #[test]
    fn rules_leave_out_the_indent_where_asked() {
        let mut letter = p("서하에게.");
        if let Block::Paragraph { attrs, .. } = &mut letter {
            attrs.left = 2;
        }
        let mut own = p("내 모양대로.");
        if let Block::Paragraph { attrs, .. } = &mut own {
            attrs.indent = Some(-1);
        }
        let blocks = vec![
            p("첫 문단."),
            p("“대사.”"),
            letter,
            Block::SceneBreak {},
            p("장면 뒤."),
            own,
            p("둘째."),
        ];
        // Korean books: only letters and quotations lose the indent.
        let korean = IndentRules {
            margined: true,
            ..IndentRules::default()
        };
        assert_eq!(
            firsts(&apply(&blocks, 1.0, korean)),
            [
                (None, 0),
                (None, 0),
                (Some(0), 2),
                (None, 0),
                (Some(-1), 0),
                (None, 0)
            ]
        );
        // English-language books: chapter openings and scene starts flush too.
        let english = IndentRules {
            chapter_first: true,
            after_scene: true,
            margined: true,
            ..IndentRules::default()
        };
        assert_eq!(
            firsts(&apply(&blocks, 1.0, english)),
            [
                (Some(0), 0),
                (None, 0),
                (Some(0), 2),
                (Some(0), 0),
                (Some(-1), 0),
                (None, 0)
            ]
        );
        // 원고지 dialogue: set in a cell as a whole.
        let hang = IndentRules {
            dialogue_hang: true,
            ..IndentRules::default()
        };
        assert_eq!(firsts(&apply(&blocks, 1.0, hang))[1], (Some(0), 1));
        // No indent in the format: nothing to leave out.
        assert_eq!(firsts(&apply(&blocks, 0.0, english))[0], (None, 0));
    }

    #[test]
    fn dialogue_marks() {
        assert!(is_dialogue(&[Inline::Text {
            text: "  “영업, 끝났나요?”".into(),
            marks: vec![]
        }]));
        assert!(is_dialogue(&[Inline::Text {
            text: "「상태창」".into(),
            marks: vec![]
        }]));
        assert!(!is_dialogue(&[Inline::Text {
            text: "서하는 말했다.".into(),
            marks: vec![]
        }]));
    }
}
