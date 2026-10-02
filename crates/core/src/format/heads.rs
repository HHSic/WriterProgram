//! 머리말 and 꼬리말: what they show and where they sit.

use serde::{Deserialize, Serialize};

/// What 머리말 shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HeadContent {
    #[default]
    None,
    /// The work's title.
    Title,
    /// The title of the chapter the page is in.
    Chapter,
    /// As in books: the work's title on even pages, the chapter's on odd ones.
    TitleChapter,
    /// The pen name.
    Author,
    /// Text the writer typed.
    Custom,
}

/// Where 머리말 sits. `Outside` puts it on the outer edge: left on even
/// pages, right on odd ones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HeadAlign {
    Left,
    #[default]
    Center,
    Right,
    Outside,
}

impl HeadAlign {
    /// Where it lands on even and odd pages.
    pub fn sides(self) -> (HeadAlign, HeadAlign) {
        match self {
            HeadAlign::Outside => (HeadAlign::Left, HeadAlign::Right),
            a => (a, a),
        }
    }

    /// Lands in the same place as `other` on some page.
    pub fn meets(self, other: HeadAlign) -> bool {
        let (a, b) = (self.sides(), other.sides());
        a.0 == b.0 || a.1 == b.1
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RunningHead {
    pub content: HeadContent,
    /// For `Custom`.
    pub text: String,
    pub align: HeadAlign,
    /// Left off the first page of each chapter, as books do. Only when
    /// chapters start on a new page.
    pub skip_chapter_first: bool,
}

impl Default for RunningHead {
    fn default() -> Self {
        RunningHead {
            content: HeadContent::None,
            text: String::new(),
            align: HeadAlign::Center,
            skip_chapter_first: true,
        }
    }
}

impl RunningHead {
    pub fn is_on(&self) -> bool {
        self.content != HeadContent::None
    }

    /// Different text or place on even and odd pages.
    pub fn facing(&self) -> bool {
        self.is_on()
            && (self.align == HeadAlign::Outside || self.content == HeadContent::TitleChapter)
    }

    /// The text of a 머리말 that is the same on every page: the pen name, the
    /// writer's own text, or else the work's title.
    pub fn fixed_text(&self, title: &str, author: &str) -> String {
        match self.content {
            HeadContent::Author => author.to_string(),
            HeadContent::Custom => self.text.clone(),
            _ => title.to_string(),
        }
    }
}

/// 꼬리말: text the writer typed, shown at the bottom of every page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RunningFoot {
    /// Empty when there is no 꼬리말.
    pub text: String,
    pub align: HeadAlign,
}

impl Default for RunningFoot {
    fn default() -> Self {
        RunningFoot {
            text: String::new(),
            align: HeadAlign::Left,
        }
    }
}

impl RunningFoot {
    pub fn is_on(&self) -> bool {
        !self.text.trim().is_empty()
    }
}
