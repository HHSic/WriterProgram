//! Manuscript body format (본문 표기).
//!
//! The editor exchanges a body as ProseMirror JSON (Tiptap `getJSON()`); the
//! types below mirror that shape. On disk the body is Markdown-compatible plain
//! text, so a manuscript stays readable in any text editor:
//!
//! - Paragraphs are separated by one empty line.
//! - A line break inside a paragraph is a plain newline. An empty line inside a
//!   paragraph is written as a single `\`.
//! - An intentionally empty paragraph is written as `&nbsp;`.
//! - A scene break is a line with exactly `***`.
//! - A paragraph set in from the sides (문단 여백) is wrapped in
//!   `<p data-left="2" data-right="1">…</p>`, in characters.
//! - Marks: `**굵게**`, `*기울임*`, `~~취소선~~`, `<u>밑줄</u>`,
//!   `<span class="dot">방점</span>`, `<mark data-memo="id">메모 구간</mark>`.
//! - `\` escapes ASCII punctuation. `\` and `*` in text are always escaped;
//!   `~` and `<` only where they would otherwise read as markup.

mod parse;
#[cfg(test)]
mod tests;
mod write;

pub use parse::parse_body;
pub use write::write_body;

use serde::{Deserialize, Deserializer, Serialize};

pub const SCENE_BREAK_LINE: &str = "***";
const EMPTY_PARAGRAPH: &str = "&nbsp;";
const EMPTY_LINE: &str = "\\";
const PARA_CLOSE: &str = "</p>";

/// Root node of an editor document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Body {
    #[serde(rename = "type", default = "doc_node_type")]
    pub node_type: String,
    #[serde(default)]
    pub content: Vec<Block>,
}

fn doc_node_type() -> String {
    "doc".into()
}

impl Body {
    pub fn new(content: Vec<Block>) -> Self {
        Body {
            node_type: doc_node_type(),
            content,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Block {
    Paragraph {
        #[serde(
            default,
            deserialize_with = "attrs_or_plain",
            skip_serializing_if = "ParaAttrs::is_plain"
        )]
        attrs: ParaAttrs,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<Inline>,
    },
    SceneBreak {},
}

/// Paragraph shape (문단 모양) in characters: margins that set the paragraph
/// in from the left and right as a whole (letters, poems, a 상태창), and the
/// paragraph's own first line: in (들여쓰기, positive), out (내어쓰기,
/// negative) or flush (0). Without one the paragraph follows the manuscript
/// format's indent and its rules (indent.rs).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct ParaAttrs {
    #[serde(default, deserialize_with = "margin")]
    pub left: u8,
    #[serde(default, deserialize_with = "margin")]
    pub right: u8,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "first_line"
    )]
    pub indent: Option<i8>,
}

impl ParaAttrs {
    /// Widest margin, in characters.
    pub const MAX: u8 = 20;
    /// Deepest first-line indent either way, in characters.
    pub const MAX_INDENT: i8 = 10;

    pub fn is_plain(&self) -> bool {
        self.left == 0 && self.right == 0 && self.indent.is_none()
    }
}

/// Reads a first-line indent leniently, like `margin`; null is none.
fn first_line<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Option<i8>, D::Error> {
    let v: Option<f64> = Option::deserialize(d)?;
    let max = f64::from(ParaAttrs::MAX_INDENT);
    Ok(v.filter(|v| v.is_finite())
        .map(|v| v.round().clamp(-max, max) as i8))
}

/// `attrs: null` reads as no margins.
fn attrs_or_plain<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<ParaAttrs, D::Error> {
    Ok(Option::<ParaAttrs>::deserialize(d)?.unwrap_or_default())
}

/// Reads a margin leniently: whatever number the editor sends is rounded and
/// kept within 0..=MAX, so an odd value never stops a save.
fn margin<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<u8, D::Error> {
    let v: Option<f64> = Option::deserialize(d)?;
    Ok(v.filter(|v| v.is_finite())
        .map_or(0, |v| v.round().clamp(0.0, f64::from(ParaAttrs::MAX)) as u8))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Inline {
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        marks: Vec<Mark>,
    },
    HardBreak {},
}

/// Variant order is the canonical nesting order, outermost first.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Mark {
    Memo { attrs: MemoAttrs },
    Underline {},
    Dot {},
    Strike {},
    Bold {},
    Italic {},
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MemoAttrs {
    pub id: String,
}

impl Mark {
    fn is_tag(&self) -> bool {
        matches!(self, Mark::Memo { .. } | Mark::Underline {} | Mark::Dot {})
    }
}

impl Block {
    /// A paragraph without margins.
    pub fn para(content: Vec<Inline>) -> Block {
        Block::Paragraph {
            attrs: ParaAttrs::default(),
            content,
        }
    }

    /// A paragraph of plain text without marks. Newlines become line breaks.
    pub fn text(s: &str) -> Block {
        let mut content = Vec::new();
        for (i, line) in s.split('\n').enumerate() {
            if i > 0 {
                content.push(Inline::HardBreak {});
            }
            if !line.is_empty() {
                content.push(Inline::Text {
                    text: line.to_string(),
                    marks: vec![],
                });
            }
        }
        Block::para(content)
    }

    /// Margins of a paragraph; none for a scene break.
    pub fn attrs(&self) -> ParaAttrs {
        match self {
            Block::Paragraph { attrs, .. } => *attrs,
            Block::SceneBreak {} => ParaAttrs::default(),
        }
    }

    /// Lines of plain text in this paragraph (split at line breaks). A scene
    /// break has no lines.
    pub fn lines(&self) -> Vec<String> {
        match self {
            Block::SceneBreak {} => Vec::new(),
            Block::Paragraph { content, .. } => {
                let mut lines = vec![String::new()];
                for inline in content {
                    match inline {
                        Inline::Text { text, .. } => {
                            for (i, part) in text.split('\n').enumerate() {
                                if i > 0 {
                                    lines.push(String::new());
                                }
                                lines.last_mut().expect("never empty").push_str(part);
                            }
                        }
                        Inline::HardBreak {} => lines.push(String::new()),
                    }
                }
                lines
            }
        }
    }
}

/// Bold, italic and strikethrough: the marks written as `**`, `*` and `~~`,
/// which switch on and off rather than open and close.
#[derive(Default, Clone, Copy, PartialEq)]
struct Toggles {
    strike: bool,
    bold: bool,
    italic: bool,
}

/// Whether `chars` begins with `s`.
fn starts_with(chars: &[char], s: &str) -> bool {
    let mut it = chars.iter();
    s.chars().all(|c| it.next() == Some(&c))
}

/// Plain text of a paragraph's content, line breaks as `\n`.
pub fn inline_text(content: &[Inline]) -> String {
    let mut text = String::new();
    for inline in content {
        match inline {
            Inline::Text { text: t, .. } => text.push_str(t),
            Inline::HardBreak {} => text.push('\n'),
        }
    }
    text
}

/// Plain text of a body: paragraphs joined by newlines, scene breaks as empty
/// lines. Used for search and previews.
pub fn plain_text(blocks: &[Block]) -> String {
    blocks
        .iter()
        .map(|b| b.lines().join("\n"))
        .collect::<Vec<_>>()
        .join("\n")
}
