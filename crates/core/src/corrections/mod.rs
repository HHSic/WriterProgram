//! 교정본 주고받기: sending chapters to an editor and taking back the
//! corrected file. The flow and the reading rules are in
//! `docs/corrections.md`.
//!
//! 1. [`send`] exports chapters (HWPX or docx) and keeps what was sent under
//!    `.exchanges/<id>/`, so the corrected file is later compared with the
//!    text as it was sent, not as it is now.
//! 2. [`read_corrected`] reads the corrected file and compares it with what
//!    was sent: tracked changes, marks the editor added (new strikethrough
//!    = delete this, new underline/colour/highlight on unchanged text =
//!    look here), and plain comparison for the rest. The result, a
//!    [`Review`], is kept as `review.json` next to the record.
//! 3. [`apply`] accepts or rejects changes, one by one or a whole class at
//!    once, on the chapter as it is now, after keeping a record
//!    (`before-corrections`) of it.

mod apply;
mod compare;
mod flat;
mod review;
mod sent;
#[cfg(test)]
mod tests;

pub use apply::{Applied, Decisions, Skipped, apply};
pub use review::{current_review, load_review, read_corrected};
pub use sent::{Exchange, ExchangeInfo, Received, SentChapter, list, load, send};

use serde::{Deserialize, Serialize};

/// Folder of the records in a project.
pub const EXCHANGE_DIR: &str = ".exchanges";

/// The kind of correction, so a whole kind can be accepted at once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeClass {
    /// 띄어쓰기: only spaces differ.
    Spacing,
    /// 문장부호: only punctuation (and spaces) differ.
    Punctuation,
    /// 문장 고침: everything else.
    Wording,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    Insert,
    Delete,
    Replace,
    /// Two paragraphs became one.
    Merge,
    /// A paragraph became two.
    Split,
}

/// How a change was found in the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Found {
    /// 변경 내용 추적 (한글) or track changes (Word), with author and date.
    Tracked,
    /// The editor struck the text through.
    Strike,
    /// New text in colour, underlined or highlighted.
    Color,
    /// The text simply differs from what was sent.
    Compared,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    #[default]
    Pending,
    Accepted,
    Rejected,
}

/// A place in a chapter: paragraph (block) index and UTF-16 offset in it,
/// as the editor counts. A scene break is one character long.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pos {
    pub block: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub from: Pos,
    pub to: Pos,
}

/// One correction. `before` and `after` use U+2029 for a paragraph break,
/// `\n` for a line break and U+E000 for a scene break.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    pub id: String,
    pub kind: ChangeKind,
    pub class: ChangeClass,
    pub before: String,
    pub after: String,
    /// A few characters before and after, to show it in a list.
    pub lead: String,
    pub trail: String,
    /// Where it is in the text that was sent.
    pub at: Span,
    /// Where it is in the chapter now; none when it cannot be placed.
    pub now: Option<Span>,
    /// 겹침: the writer changed the paragraph after sending it too, so the
    /// change is not applied without the writer looking at it.
    pub overlap: bool,
    pub how: Found,
    /// Who made a tracked change, and when.
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub state: State,
}

/// 살펴볼 곳: unchanged text the editor underlined, coloured or highlighted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Look {
    pub id: String,
    pub at: Span,
    pub now: Option<Span>,
    pub quote: String,
    pub underline: bool,
    pub color: bool,
    pub highlight: bool,
}

/// A note the editor left (한글 메모, Word comment).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditorNote {
    pub id: String,
    pub text: String,
    pub author: String,
    pub date: String,
    /// The text it is on, in the text that was sent; none for a note on no
    /// particular text.
    pub at: Option<Span>,
    pub now: Option<Span>,
    /// The text it is on, as in the corrected file.
    pub quote: String,
    /// Accepted: made into a 메모 (`memo`); rejected: left out.
    #[serde(default)]
    pub state: State,
    #[serde(default)]
    pub memo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterReview {
    pub doc_id: String,
    pub title: String,
    /// The chapter's text is in the corrected file.
    pub found: bool,
    /// The chapter was deleted since it was sent.
    pub gone: bool,
    /// The writer changed the chapter since it was sent.
    pub edited: bool,
    pub changes: Vec<Change>,
    pub looks: Vec<Look>,
    pub notes: Vec<EditorNote>,
}

/// What a corrected file says, chapter by chapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Review {
    pub exchange: String,
    /// The corrected file's name, and the copy kept with the record.
    pub file: String,
    pub stored: String,
    pub at: String,
    pub chapters: Vec<ChapterReview>,
}

impl Review {
    /// Changes not yet accepted or rejected.
    pub fn pending(&self) -> usize {
        self.chapters
            .iter()
            .flat_map(|c| &c.changes)
            .filter(|c| c.state == State::Pending)
            .count()
    }
}
