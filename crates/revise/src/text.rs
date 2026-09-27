//! Paragraph and sentence segmentation. Offsets are byte offsets into the whole text.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Narration,
    Dialogue,
    Thought,
}

#[derive(Clone, Debug)]
pub struct Paragraph {
    pub index: usize,
    pub line: usize,
    pub start: usize,
    pub end: usize,
    /// Status windows, scene breaks and similar blocks are not checked.
    pub excluded: bool,
}

#[derive(Clone, Debug)]
pub struct Sentence {
    pub para: usize,
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
}

pub fn paragraphs(text: &str) -> Vec<Paragraph> {
    let mut out = Vec::new();
    let mut offset = 0;
    for (line_no, line) in text.split('\n').enumerate() {
        let start = offset;
        offset = start + line.len() + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let lead = line.len() - line.trim_start().len();
        out.push(Paragraph {
            index: out.len(),
            line: line_no + 1,
            start: start + lead,
            end: start + lead + trimmed.len(),
            excluded: is_excluded(trimmed),
        });
    }
    out
}

fn is_excluded(p: &str) -> bool {
    let wrapped = |open: char, close: char| p.starts_with(open) && p.ends_with(close);
    if wrapped('[', ']') || wrapped('【', '】') || wrapped('<', '>') || wrapped('〈', '〉') {
        return true;
    }
    // Scene breaks such as ◆, ***, - - -
    p.chars()
        .all(|c| matches!(c, '◆' | '◇' | '*' | '-' | '―' | '─' | '•' | '·') || c.is_whitespace())
}

fn closing_quote(c: char) -> Option<(char, Kind)> {
    match c {
        '“' => Some(('”', Kind::Dialogue)),
        '「' => Some(('」', Kind::Dialogue)),
        '『' => Some(('』', Kind::Dialogue)),
        '"' => Some(('"', Kind::Dialogue)),
        '‘' => Some(('’', Kind::Thought)),
        _ => None,
    }
}

fn is_terminal(c: char) -> bool {
    matches!(c, '.' | '?' | '!' | '…' | '。')
}

pub fn sentences(text: &str, p: &Paragraph) -> Vec<Sentence> {
    let mut out = Vec::new();
    if p.excluded {
        return out;
    }
    let body = &text[p.start..p.end];
    let mut narration_start = p.start;
    let mut open: Option<(char, Kind, usize)> = None;

    for (i, c) in body.char_indices() {
        let at = p.start + i;
        match open {
            None => {
                if let Some((close, kind)) = closing_quote(c) {
                    split_narration(text, p.index, narration_start, at, &mut out);
                    open = Some((close, kind, at));
                }
            }
            Some((close, kind, quote_start)) => {
                if c == close {
                    let end = at + c.len_utf8();
                    out.push(Sentence { para: p.index, start: quote_start, end, kind });
                    open = None;
                    narration_start = end;
                }
            }
        }
    }
    match open {
        Some((_, kind, quote_start)) => {
            out.push(Sentence { para: p.index, start: quote_start, end: p.end, kind })
        }
        None => split_narration(text, p.index, narration_start, p.end, &mut out),
    }
    out
}

fn split_narration(text: &str, para: usize, start: usize, end: usize, out: &mut Vec<Sentence>) {
    let region = &text[start..end];
    let chars: Vec<(usize, char)> = region.char_indices().collect();
    let mut sent_start: Option<usize> = None;
    let mut i = 0;
    while i < chars.len() {
        let (off, c) = chars[i];
        if sent_start.is_none() && !c.is_whitespace() {
            sent_start = Some(off);
        }
        if is_terminal(c) {
            let mut j = i + 1;
            while j < chars.len() && (is_terminal(chars[j].1) || matches!(chars[j].1, ')' | '」' | '”' | '’')) {
                j += 1;
            }
            let at_boundary = j >= chars.len() || chars[j].1.is_whitespace();
            if at_boundary {
                let s_end = if j < chars.len() { chars[j].0 } else { region.len() };
                if let Some(s) = sent_start.take() {
                    out.push(Sentence { para, start: start + s, end: start + s_end, kind: Kind::Narration });
                }
            }
            i = j;
            continue;
        }
        i += 1;
    }
    if let Some(s) = sent_start {
        let tail = region[s..].trim_end();
        if !tail.is_empty() {
            out.push(Sentence { para, start: start + s, end: start + s + tail.len(), kind: Kind::Narration });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_dialogue_and_narration() {
        let text = "“이거, 어디서 났어?” 서하가 물었다. 남자는 웃었다.";
        let ps = paragraphs(text);
        let ss = sentences(text, &ps[0]);
        let kinds: Vec<Kind> = ss.iter().map(|s| s.kind).collect();
        assert_eq!(kinds, vec![Kind::Dialogue, Kind::Narration, Kind::Narration]);
        assert_eq!(&text[ss[1].start..ss[1].end], "서하가 물었다.");
    }

    #[test]
    fn excludes_status_windows_and_scene_breaks() {
        let text = "[알림: 기록이 갱신되었습니다]\n◆\n본문이다.";
        let ps = paragraphs(text);
        assert!(ps[0].excluded && ps[1].excluded && !ps[2].excluded);
    }
}
