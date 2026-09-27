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
//! - Marks: `**굵게**`, `*기울임*`, `~~취소선~~`, `<u>밑줄</u>`,
//!   `<span class="dot">방점</span>`, `<mark data-memo="id">메모 구간</mark>`.
//! - `\` escapes ASCII punctuation. `\` and `*` in text are always escaped;
//!   `~` and `<` only where they would otherwise read as markup.

use serde::{Deserialize, Serialize};

pub const SCENE_BREAK_LINE: &str = "***";
const EMPTY_PARAGRAPH: &str = "&nbsp;";
const EMPTY_LINE: &str = "\\";

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
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        content: Vec<Inline>,
    },
    SceneBreak {},
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
        Block::Paragraph { content }
    }

    /// Lines of plain text in this paragraph (split at line breaks). A scene
    /// break has no lines.
    pub fn lines(&self) -> Vec<String> {
        match self {
            Block::SceneBreak {} => Vec::new(),
            Block::Paragraph { content } => {
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

// ---------------------------------------------------------------------------
// Writing

/// Writes blocks as body text. The result ends with a newline unless empty.
pub fn write_body(blocks: &[Block]) -> String {
    let mut out = String::new();
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            out.push_str("\n\n");
        }
        match block {
            Block::SceneBreak {} => out.push_str(SCENE_BREAK_LINE),
            Block::Paragraph { content } => out.push_str(&write_paragraph(content)),
        }
    }
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

enum Run {
    Text(String, Vec<Mark>),
    Break,
}

/// Merges neighbouring text with the same marks, turns stray newlines into line
/// breaks and drops empty text.
fn runs(content: &[Inline]) -> Vec<Run> {
    let mut out: Vec<Run> = Vec::new();
    for inline in content {
        match inline {
            Inline::HardBreak {} => out.push(Run::Break),
            Inline::Text { text, marks } => {
                let mut marks = marks.clone();
                marks.sort();
                marks.dedup();
                for (i, part) in text.replace('\r', "").split('\n').enumerate() {
                    if i > 0 {
                        out.push(Run::Break);
                    }
                    if part.is_empty() {
                        continue;
                    }
                    match out.last_mut() {
                        Some(Run::Text(prev, prev_marks)) if *prev_marks == marks => {
                            prev.push_str(part)
                        }
                        _ => out.push(Run::Text(part.to_string(), marks.clone())),
                    }
                }
            }
        }
    }
    out
}

enum Tok<'a> {
    Text(&'a str),
    Markup(String),
    Break,
}

#[derive(Default, Clone, Copy, PartialEq)]
struct Toggles {
    strike: bool,
    bold: bool,
    italic: bool,
}

impl Toggles {
    fn from_marks(marks: &[Mark]) -> Self {
        Toggles {
            strike: marks.contains(&Mark::Strike {}),
            bold: marks.contains(&Mark::Bold {}),
            italic: marks.contains(&Mark::Italic {}),
        }
    }

    /// Emits the toggles that change towards `want`, only those turning on
    /// (`on == true`) or only those turning off.
    fn change(&mut self, want: Toggles, on: bool, toks: &mut Vec<Tok>) {
        let mut order = [
            (&mut self.strike, want.strike, "~~"),
            (&mut self.bold, want.bold, "**"),
            (&mut self.italic, want.italic, "*"),
        ];
        if !on {
            order.reverse();
        }
        for (state, target, marker) in order {
            if *state != target && target == on {
                *state = target;
                toks.push(Tok::Markup(marker.to_string()));
            }
        }
    }
}

fn open_tag(mark: &Mark) -> String {
    match mark {
        Mark::Memo { attrs } => format!("<mark data-memo=\"{}\">", memo_id(&attrs.id)),
        Mark::Underline {} => "<u>".into(),
        Mark::Dot {} => "<span class=\"dot\">".into(),
        _ => unreachable!("not a tag mark"),
    }
}

fn close_tag(mark: &Mark) -> String {
    match mark {
        Mark::Memo { .. } => "</mark>".into(),
        Mark::Underline {} => "</u>".into(),
        Mark::Dot {} => "</span>".into(),
        _ => unreachable!("not a tag mark"),
    }
}

/// Memo ids are generated by the app; keep only characters that are safe
/// inside the attribute.
fn memo_id(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(64)
        .collect()
}

fn write_paragraph(content: &[Inline]) -> String {
    let runs = runs(content);
    if runs.is_empty() {
        return EMPTY_PARAGRAPH.to_string();
    }
    let mut toks: Vec<Tok> = Vec::new();
    let mut on = Toggles::default();
    let mut tags: Vec<Mark> = Vec::new();
    for run in &runs {
        match run {
            Run::Break => toks.push(Tok::Break),
            Run::Text(text, marks) => {
                let want = Toggles::from_marks(marks);
                let want_tags: Vec<&Mark> = marks.iter().filter(|m| m.is_tag()).collect();
                on.change(want, false, &mut toks);
                let keep = tags
                    .iter()
                    .zip(&want_tags)
                    .take_while(|(a, b)| a == *b)
                    .count();
                while tags.len() > keep {
                    let mark = tags.pop().expect("len > keep");
                    toks.push(Tok::Markup(close_tag(&mark)));
                }
                for mark in &want_tags[keep..] {
                    toks.push(Tok::Markup(open_tag(mark)));
                    tags.push((*mark).clone());
                }
                on.change(want, true, &mut toks);
                toks.push(Tok::Text(text));
            }
        }
    }
    on.change(Toggles::default(), false, &mut toks);
    while let Some(mark) = tags.pop() {
        toks.push(Tok::Markup(close_tag(&mark)));
    }

    let rendered = render(&toks);
    let text = rendered
        .split('\n')
        .map(|line| if line.is_empty() { EMPTY_LINE } else { line })
        .collect::<Vec<_>>()
        .join("\n");
    if text == EMPTY_PARAGRAPH {
        format!("\\{text}")
    } else {
        text
    }
}

fn render(toks: &[Tok]) -> String {
    let mut out = String::new();
    for (i, tok) in toks.iter().enumerate() {
        match tok {
            Tok::Markup(s) => out.push_str(s),
            Tok::Break => out.push('\n'),
            Tok::Text(text) => {
                let next_after = toks.get(i + 1).and_then(|t| match t {
                    Tok::Markup(s) => s.chars().next(),
                    Tok::Text(s) => s.chars().next(),
                    Tok::Break => Some('\n'),
                });
                escape_into(text, next_after, &mut out);
            }
        }
    }
    out
}

fn escape_into(text: &str, next_after: Option<char>, out: &mut String) {
    let chars: Vec<char> = text.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let next = chars.get(i + 1).copied().or(next_after);
        match c {
            '\\' => out.push_str("\\\\"),
            '*' => out.push_str("\\*"),
            '~' if next == Some('~') => out.push_str("\\~"),
            '<' if looks_like_tag(&chars[i + 1..]) => out.push_str("\\<"),
            _ => out.push(c),
        }
    }
}

/// Whether the text after a `<` could be read as one of our tags.
fn looks_like_tag(rest: &[char]) -> bool {
    const STARTS: [&str; 6] = ["u>", "/u>", "span", "/span>", "mark", "/mark>"];
    STARTS.iter().any(|s| starts_with(rest, s))
}

fn starts_with(chars: &[char], s: &str) -> bool {
    let mut it = chars.iter();
    s.chars().all(|c| it.next() == Some(&c))
}

// ---------------------------------------------------------------------------
// Reading

/// Reads body text back into blocks. Accepts CRLF-free text (see
/// `store::read_text`). Lenient with hand edits: several empty lines count as
/// one paragraph break, and unbalanced marks end with their paragraph.
pub fn parse_body(src: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut chunk: Vec<&str> = Vec::new();
    for line in src.split('\n') {
        if line.is_empty() {
            if !chunk.is_empty() {
                blocks.push(parse_chunk(&chunk));
                chunk.clear();
            }
        } else {
            chunk.push(line);
        }
    }
    if !chunk.is_empty() {
        blocks.push(parse_chunk(&chunk));
    }
    blocks
}

fn parse_chunk(lines: &[&str]) -> Block {
    if lines.len() == 1 {
        if lines[0].trim() == SCENE_BREAK_LINE {
            return Block::SceneBreak {};
        }
        if lines[0] == EMPTY_PARAGRAPH {
            return Block::Paragraph { content: vec![] };
        }
    }
    let mut p = InlineParser::default();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            p.hard_break();
        }
        if *line != EMPTY_LINE {
            p.line(line);
        }
    }
    Block::Paragraph {
        content: p.finish(),
    }
}

enum TagOp {
    Open(Mark),
    CloseUnderline,
    CloseDot,
    CloseMemo,
}

/// A tag and what reading it does.
type TagRule = (&'static str, fn() -> TagOp);

fn match_tag(chars: &[char]) -> Option<(TagOp, usize)> {
    let fixed: [TagRule; 5] = [
        ("<u>", || TagOp::Open(Mark::Underline {})),
        ("</u>", || TagOp::CloseUnderline),
        ("<span class=\"dot\">", || TagOp::Open(Mark::Dot {})),
        ("</span>", || TagOp::CloseDot),
        ("</mark>", || TagOp::CloseMemo),
    ];
    for (tag, op) in fixed {
        if starts_with(chars, tag) {
            return Some((op(), tag.chars().count()));
        }
    }
    const MEMO: &str = "<mark data-memo=\"";
    if starts_with(chars, MEMO) {
        let start = MEMO.chars().count();
        let mut id = String::new();
        for (offset, &c) in chars[start..].iter().enumerate().take(65) {
            if c == '"' {
                let end = start + offset;
                if chars.get(end + 1) == Some(&'>') && !id.is_empty() {
                    return Some((
                        TagOp::Open(Mark::Memo {
                            attrs: MemoAttrs { id },
                        }),
                        end + 2,
                    ));
                }
                return None;
            }
            if c == '<' || c == '>' {
                return None;
            }
            id.push(c);
        }
    }
    None
}

#[derive(Default)]
struct InlineParser {
    out: Vec<Inline>,
    buf: String,
    on: Toggles,
    tags: Vec<Mark>,
}

impl InlineParser {
    fn marks(&self) -> Vec<Mark> {
        let mut marks = self.tags.clone();
        if self.on.strike {
            marks.push(Mark::Strike {});
        }
        if self.on.bold {
            marks.push(Mark::Bold {});
        }
        if self.on.italic {
            marks.push(Mark::Italic {});
        }
        marks.sort();
        marks.dedup();
        marks
    }

    fn flush(&mut self) {
        if self.buf.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.buf);
        let marks = self.marks();
        match self.out.last_mut() {
            Some(Inline::Text {
                text: prev,
                marks: prev_marks,
            }) if *prev_marks == marks => prev.push_str(&text),
            _ => self.out.push(Inline::Text { text, marks }),
        }
    }

    fn hard_break(&mut self) {
        self.flush();
        self.out.push(Inline::HardBreak {});
    }

    fn remove_last(&mut self, pred: impl Fn(&Mark) -> bool) {
        if let Some(pos) = self.tags.iter().rposition(pred) {
            self.tags.remove(pos);
        }
    }

    fn line(&mut self, line: &str) {
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            let next = chars.get(i + 1).copied();
            match c {
                '\\' if next.is_some_and(|n| n.is_ascii_punctuation()) => {
                    self.buf.push(next.expect("checked"));
                    i += 2;
                }
                '*' if next == Some('*') => {
                    self.flush();
                    self.on.bold = !self.on.bold;
                    i += 2;
                }
                '*' => {
                    self.flush();
                    self.on.italic = !self.on.italic;
                    i += 1;
                }
                '~' if next == Some('~') => {
                    self.flush();
                    self.on.strike = !self.on.strike;
                    i += 2;
                }
                '<' => match match_tag(&chars[i..]) {
                    Some((op, len)) => {
                        self.flush();
                        match op {
                            TagOp::Open(mark) => self.tags.push(mark),
                            TagOp::CloseUnderline => self.remove_last(|m| *m == Mark::Underline {}),
                            TagOp::CloseDot => self.remove_last(|m| *m == Mark::Dot {}),
                            TagOp::CloseMemo => {
                                self.remove_last(|m| matches!(m, Mark::Memo { .. }))
                            }
                        }
                        i += len;
                    }
                    None => {
                        self.buf.push('<');
                        i += 1;
                    }
                },
                _ => {
                    self.buf.push(c);
                    i += 1;
                }
            }
        }
    }

    fn finish(mut self) -> Vec<Inline> {
        self.flush();
        self.out
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn t(text: &str, marks: &[Mark]) -> Inline {
        Inline::Text {
            text: text.into(),
            marks: marks.to_vec(),
        }
    }
    fn p(content: Vec<Inline>) -> Block {
        Block::Paragraph { content }
    }
    fn br() -> Inline {
        Inline::HardBreak {}
    }
    fn memo(id: &str) -> Mark {
        Mark::Memo {
            attrs: MemoAttrs { id: id.into() },
        }
    }
    const B: Mark = Mark::Bold {};
    const I: Mark = Mark::Italic {};
    const S: Mark = Mark::Strike {};
    const U: Mark = Mark::Underline {};
    const D: Mark = Mark::Dot {};

    /// Canonical form the parser produces: merged runs, sorted marks.
    fn canon(blocks: &[Block]) -> Vec<Block> {
        blocks
            .iter()
            .map(|b| match b {
                Block::SceneBreak {} => Block::SceneBreak {},
                Block::Paragraph { content } => {
                    let mut out: Vec<Inline> = Vec::new();
                    for r in runs(content) {
                        match r {
                            Run::Break => out.push(br()),
                            Run::Text(text, marks) => out.push(Inline::Text { text, marks }),
                        }
                    }
                    p(out)
                }
            })
            .collect()
    }

    fn round_trip(blocks: Vec<Block>) -> String {
        let text = write_body(&blocks);
        assert_eq!(parse_body(&text), canon(&blocks), "text was:\n{text}");
        text
    }

    #[test]
    fn plain_paragraphs() {
        let text = round_trip(vec![
            Block::text("셔터를 반쯤 내렸을 때 종이 울렸다."),
            Block::text("“영업, 끝났나요?”"),
        ]);
        assert_eq!(
            text,
            "셔터를 반쯤 내렸을 때 종이 울렸다.\n\n“영업, 끝났나요?”\n"
        );
    }

    #[test]
    fn scene_breaks_and_empty_paragraphs() {
        let text = round_trip(vec![
            Block::text("앞 장면"),
            Block::SceneBreak {},
            p(vec![]),
            Block::text("뒤 장면"),
        ]);
        assert_eq!(text, "앞 장면\n\n***\n\n&nbsp;\n\n뒤 장면\n");
    }

    #[test]
    fn line_breaks_inside_paragraph() {
        let text = round_trip(vec![p(vec![t("첫 줄", &[]), br(), t("둘째 줄", &[])])]);
        assert_eq!(text, "첫 줄\n둘째 줄\n");
        let text = round_trip(vec![p(vec![
            br(),
            t("가", &[]),
            br(),
            br(),
            t("나", &[]),
            br(),
        ])]);
        assert_eq!(text, "\\\n가\n\\\n나\n\\\n");
        round_trip(vec![p(vec![br()])]);
    }

    #[test]
    fn marks() {
        let text = round_trip(vec![p(vec![
            t("그녀는 ", &[]),
            t("굵게", &[B]),
            t("와 ", &[]),
            t("기울임", &[I]),
            t(", ", &[]),
            t("취소", &[S]),
            t(", ", &[]),
            t("밑줄", &[U]),
            t(", ", &[]),
            t("방점", &[D]),
        ])]);
        assert_eq!(
            text,
            "그녀는 **굵게**와 *기울임*, ~~취소~~, <u>밑줄</u>, <span class=\"dot\">방점</span>\n"
        );
    }

    #[test]
    fn overlapping_marks() {
        round_trip(vec![p(vec![
            t("a", &[B]),
            t("b", &[B, I]),
            t("c", &[I]),
            t("d", &[U, I]),
            t("e", &[U, D, B]),
            t("f", &[D]),
        ])]);
        round_trip(vec![p(vec![
            t("굵게", &[B, I]),
            t("기울임", &[I]),
            t("굵게", &[B]),
        ])]);
        round_trip(vec![p(vec![t("x", &[B]), br(), t("y", &[B]), t("z", &[])])]);
    }

    #[test]
    fn memo_anchors() {
        let text = round_trip(vec![p(vec![
            t("앞 ", &[]),
            t("구간", &[memo("m1")]),
            t("겹침", &[memo("m1"), memo("m2")]),
            t("끝", &[memo("m2"), B]),
        ])]);
        assert!(text.starts_with("앞 <mark data-memo=\"m1\">구간<mark data-memo=\"m2\">"));
    }

    #[test]
    fn escapes() {
        for s in [
            "별*표",
            "***",
            "* * *",
            "\\",
            "a\\b",
            "물결~",
            "~~",
            "~~~",
            "그래~ 알았어~~",
            "&nbsp;",
            "<u>태그</u> 아님",
            "<span class=\"dot\">",
            "<mark data-memo=\"x\">",
            "<상태창> 레벨 업",
            "a < b > c",
            "\\*",
            " 앞 공백",
            "뒤 공백 ",
            "   ",
        ] {
            round_trip(vec![Block::text(s)]);
            round_trip(vec![p(vec![t(s, &[B])])]);
            round_trip(vec![p(vec![t(s, &[S]), t(s, &[]), t(s, &[U, S])])]);
        }
    }

    #[test]
    fn web_novel_brackets_stay_readable() {
        let text = round_trip(vec![Block::text("<상태창>"), Block::text("[레벨 업!]")]);
        assert_eq!(text, "<상태창>\n\n[레벨 업!]\n");
    }

    #[test]
    fn lenient_with_hand_edits() {
        let blocks = parse_body("첫 문단\n\n\n\n둘째 문단\n  ***  \n");
        assert_eq!(blocks.len(), 2);
        assert_eq!(parse_body("  ***  "), vec![Block::SceneBreak {}]);
        // An unclosed mark ends with its paragraph.
        assert_eq!(
            parse_body("**열림\n\n닫힘"),
            vec![p(vec![t("열림", &[B])]), Block::text("닫힘")]
        );
        // Unknown backslash sequences stay as they are.
        assert_eq!(parse_body("a\\가"), vec![Block::text("a\\가")]);
    }

    #[test]
    fn json_matches_tiptap() {
        let json = r#"{"type":"doc","content":[
            {"type":"paragraph","content":[
                {"type":"text","text":"굵게","marks":[{"type":"bold"}]},
                {"type":"hardBreak"},
                {"type":"text","text":"메모","marks":[{"type":"memo","attrs":{"id":"m1"}}]}
            ]},
            {"type":"sceneBreak"},
            {"type":"paragraph"}
        ]}"#;
        let body: Body = serde_json::from_str(json).unwrap();
        assert_eq!(body.content.len(), 3);
        let back = serde_json::to_value(&body).unwrap();
        assert_eq!(
            back,
            serde_json::from_str::<serde_json::Value>(json).unwrap()
        );
    }

    /// Random bodies over a tricky alphabet must survive a write and read.
    #[test]
    fn random_round_trips() {
        let alphabet: Vec<char> = "가나 다*~<>\\/&;nbspumarkd=\"\n.".chars().collect();
        let all_marks = [B, I, S, U, D, memo("a"), memo("b")];
        let mut seed: u64 = 0x2545_f491_4f6c_dd1d;
        let mut next = move |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        for _ in 0..3000 {
            let mut blocks = Vec::new();
            for _ in 0..next(4) + 1 {
                if next(8) == 0 {
                    blocks.push(Block::SceneBreak {});
                    continue;
                }
                let mut content = Vec::new();
                for _ in 0..next(5) {
                    if next(6) == 0 {
                        content.push(br());
                        continue;
                    }
                    let text: String = (0..next(6) + 1)
                        .map(|_| alphabet[next(alphabet.len())])
                        .collect();
                    let marks: Vec<Mark> =
                        all_marks.iter().filter(|_| next(4) == 0).cloned().collect();
                    content.push(t(&text, &marks));
                }
                blocks.push(p(content));
            }
            round_trip(blocks);
        }
    }
}
