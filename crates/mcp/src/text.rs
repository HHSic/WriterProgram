//! Plain text in and out of a manuscript body, for tools that speak text.
//!
//! One paragraph per line and `***` for a scene break: what a language model
//! writes naturally, and what survives a round trip. Line breaks inside a
//! paragraph come out as separate lines and go back in as paragraphs, which
//! novels rarely need.

use writer_core::markup::{Block, Inline};

pub const SCENE_BREAK: &str = "***";

/// Each non-empty line is a paragraph; a line of exactly `***` is a scene
/// break. Empty lines only separate.
pub fn to_blocks(text: &str) -> Vec<Block> {
    text.replace("\r\n", "\n")
        .split('\n')
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            if line.trim() == SCENE_BREAK {
                Block::SceneBreak {}
            } else {
                Block::text(line)
            }
        })
        .collect()
}

/// Paragraphs separated by an empty line, scene breaks as `***`.
pub fn to_text(blocks: &[Block]) -> String {
    blocks
        .iter()
        .map(|b| match b {
            Block::SceneBreak {} => SCENE_BREAK.to_string(),
            Block::Paragraph { .. } => b.lines().join("\n"),
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Whether the body has anything plain text cannot carry: marks (bold, 방점,
/// memo stretches and so on) or paragraph margins and alignment. Replacing
/// such a body with plain text would lose them.
pub fn has_formatting(blocks: &[Block]) -> bool {
    blocks.iter().any(|b| match b {
        Block::SceneBreak {} => false,
        Block::Paragraph { attrs, content } => {
            !attrs.is_plain()
                || content
                    .iter()
                    .any(|i| matches!(i, Inline::Text { marks, .. } if !marks.is_empty()))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use writer_core::markup::parse_body;

    #[test]
    fn lines_become_paragraphs_and_back() {
        let blocks = to_blocks("문이 열렸다.\n\n“누구세요?”\n***\n\r\n비가 왔다.  \n");
        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[2], Block::SceneBreak {});
        let text = to_text(&blocks);
        assert_eq!(text, "문이 열렸다.\n\n“누구세요?”\n\n***\n\n비가 왔다.");
        assert_eq!(to_blocks(&text), blocks);
    }

    #[test]
    fn markup_characters_stay_text() {
        let blocks = to_blocks("별 *세 개* 와 <u>밑줄</u>");
        assert!(!has_formatting(&blocks));
        assert_eq!(to_text(&blocks), "별 *세 개* 와 <u>밑줄</u>");
    }

    #[test]
    fn marks_and_margins_count_as_formatting() {
        assert!(has_formatting(&parse_body("그는 **분명히** 말했다.")));
        assert!(has_formatting(&parse_body(
            "<p data-left=\"2\">들여 쓴 문단</p>"
        )));
        assert!(!has_formatting(&parse_body(
            "그냥 문단\n\n***\n\n다음 문단"
        )));
    }
}
