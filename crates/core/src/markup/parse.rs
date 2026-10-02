//! Reading body text back into blocks.

use super::{
    Block, EMPTY_LINE, EMPTY_PARAGRAPH, Inline, Mark, MemoAttrs, PARA_CLOSE, ParaAttrs,
    SCENE_BREAK_LINE, Toggles, starts_with,
};

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

/// Reads `<p data-left="2" data-right="1" data-indent="-1">` at the start of
/// a paragraph: the shape and the length of the tag in chars.
pub(super) fn parse_para_open(chars: &[char]) -> Option<(ParaAttrs, usize)> {
    if !starts_with(chars, "<p") {
        return None;
    }
    let mut attrs = ParaAttrs::default();
    let mut i = 2;
    loop {
        if chars.get(i) == Some(&'>') {
            return Some((attrs, i + 1));
        }
        let rest = &chars[i.min(chars.len())..];
        let (key, len) = if starts_with(rest, " data-left=\"") {
            ('l', " data-left=\"".len())
        } else if starts_with(rest, " data-right=\"") {
            ('r', " data-right=\"".len())
        } else if starts_with(rest, " data-indent=\"") {
            ('i', " data-indent=\"".len())
        } else {
            return None;
        };
        i += len;
        let negative = key == 'i' && chars.get(i) == Some(&'-');
        if negative {
            i += 1;
        }
        let digits: String = chars[i.min(chars.len())..]
            .iter()
            .take_while(|c| c.is_ascii_digit())
            .take(3)
            .collect();
        if digits.is_empty() || chars.get(i + digits.len()) != Some(&'"') {
            return None;
        }
        let n = digits.parse::<u8>().unwrap_or(u8::MAX);
        match key {
            'l' => attrs.left = n.min(ParaAttrs::MAX),
            'r' => attrs.right = n.min(ParaAttrs::MAX),
            _ => {
                let n = n.min(ParaAttrs::MAX_INDENT as u8) as i8;
                attrs.indent = Some(if negative { -n } else { n });
            }
        }
        i += digits.len() + 1;
    }
}

fn parse_chunk(lines: &[&str]) -> Block {
    if lines.len() == 1 {
        if lines[0].trim() == SCENE_BREAK_LINE {
            return Block::SceneBreak {};
        }
        if lines[0] == EMPTY_PARAGRAPH {
            return Block::para(vec![]);
        }
    }
    // A paragraph with margins: the tag opens its first line and `</p>` ends
    // its last one (a missing end is forgiven, as with other hand edits).
    let first: Vec<char> = lines[0].chars().collect();
    let (attrs, lines) = match parse_para_open(&first) {
        Some((attrs, len)) => {
            let mut inner: Vec<String> = lines.iter().map(|l| (*l).to_string()).collect();
            inner[0] = first[len..].iter().collect();
            let last = inner.len() - 1;
            if let Some(stripped) = inner[last].strip_suffix(PARA_CLOSE) {
                inner[last] = stripped.to_string();
            }
            if inner.len() == 1 && inner[0].is_empty() {
                inner.clear();
            }
            (attrs, inner)
        }
        None => (
            ParaAttrs::default(),
            lines.iter().map(|l| (*l).to_string()).collect(),
        ),
    };
    let mut p = InlineParser::default();
    for (i, line) in lines.iter().enumerate() {
        if i > 0 {
            p.hard_break();
        }
        if line != EMPTY_LINE {
            p.line(line);
        }
    }
    Block::Paragraph {
        attrs,
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
