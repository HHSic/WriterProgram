//! Paragraphs and sentences of a manuscript, dialogue (inside quotes) told
//! apart from narration. Characters only, no morphology. Lines count from 1.
//! Port of `plugins/kfiction/lib/text.js`.

use std::sync::LazyLock;

use regex::Regex;

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).expect("pattern compiles")
}

static BREAK: LazyLock<Regex> = LazyLock::new(|| re(r"^[\s◆◇*\-―─•·~=]+$"));
static HEADING: LazyLock<Regex> = LazyLock::new(|| re(r"^#{1,6}\s"));
static STATUS: LazyLock<Regex> = LazyLock::new(|| re(r"^\[.*\]$|^【.*】$"));
static CODE: LazyLock<Regex> = LazyLock::new(|| re(r"(?s)```.*?```|~~~.*?~~~"));
static SPACES: LazyLock<Regex> = LazyLock::new(|| re(r"\s+"));
static SENTENCE: LazyLock<Regex> = LazyLock::new(|| re(r"[^.?!…]+[.?!…]*"));
static TRAILING_MARKS: LazyLock<Regex> = LazyLock::new(|| re(r"[^가-힣A-Za-z0-9\s]+$"));
static TRAILING_NON_HANGUL: LazyLock<Regex> = LazyLock::new(|| re(r"[^가-힣]+$"));
static TWO_HANGUL: LazyLock<Regex> = LazyLock::new(|| re(r"[가-힣]{2}"));

pub fn is_hangul(c: char) -> bool {
    ('가'..='힣').contains(&c)
}

pub fn has_hangul(s: &str) -> bool {
    s.chars().any(is_hangul)
}

/// Length as the JS checker counts it. Korean text is all in the basic
/// plane, where UTF-16 units and characters agree.
pub fn len(s: &str) -> usize {
    s.chars().count()
}

/// Keeps only the newlines of `s`, so line numbers stay put.
fn blank(s: &str) -> String {
    s.chars().filter(|&c| c == '\n').collect()
}

/// Takes out front matter and code blocks, keeping the line count.
pub fn strip_non_prose(text: &str) -> String {
    let text = text
        .strip_prefix('\u{feff}')
        .unwrap_or(text)
        .replace("\r\n", "\n");
    let mut text = text;
    if text.starts_with("---\n")
        && let Some(at) = text[4..].find("\n---\n")
    {
        let end = 4 + at + 4;
        text = blank(&text[..end]) + &text[end..];
    }
    CODE.replace_all(&text, |c: &regex::Captures| blank(&c[0]))
        .into_owned()
}

#[derive(Debug, Clone)]
pub struct Paragraph {
    pub line: usize,
    pub text: String,
    /// Scene breaks, status windows (`[ ]`, `【 】`) and headings.
    pub excluded: bool,
    pub dialogues: Vec<String>,
    pub narration: String,
    pub dialogue_only: bool,
}

pub fn paragraphs(text: &str) -> Vec<Paragraph> {
    strip_non_prose(text)
        .split('\n')
        .enumerate()
        .filter_map(|(i, raw)| {
            let t = raw.trim();
            if t.is_empty() {
                return None;
            }
            let excluded = BREAK.is_match(t) || HEADING.is_match(t) || STATUS.is_match(t);
            let (dialogues, narration) = split_quotes(t);
            let dialogue_only = !dialogues.is_empty() && !TWO_HANGUL.is_match(&narration);
            Some(Paragraph {
                line: i + 1,
                text: t.to_string(),
                excluded,
                dialogues,
                narration,
                dialogue_only,
            })
        })
        .collect()
}

fn closing(c: char) -> Option<char> {
    match c {
        '“' => Some('”'),
        '「' => Some('」'),
        '『' => Some('』'),
        '"' => Some('"'),
        _ => None,
    }
}

/// Dialogue (inside quotes) and narration (outside). A quote without its
/// pair runs to the end of the paragraph.
pub fn split_quotes(t: &str) -> (Vec<String>, String) {
    let chars: Vec<char> = t.chars().collect();
    let mut dialogues = Vec::new();
    let mut narration = String::new();
    let mut i = 0;
    while i < chars.len() {
        if let Some(close) = closing(chars[i]) {
            let stop = chars[i + 1..]
                .iter()
                .position(|&c| c == close)
                .map_or(chars.len(), |p| i + 1 + p);
            dialogues.push(
                chars[i + 1..stop]
                    .iter()
                    .collect::<String>()
                    .trim()
                    .to_string(),
            );
            narration.push(' ');
            i = stop + 1;
        } else {
            narration.push(chars[i]);
            i += 1;
        }
    }
    let narration = SPACES.replace_all(&narration, " ").trim().to_string();
    (dialogues, narration)
}

/// Sentences of narration. Runs of `.?!…` end one sentence.
pub fn sentences(s: &str) -> Vec<String> {
    SENTENCE
        .find_iter(s)
        .map(|m| m.as_str().trim().to_string())
        .filter(|x| has_hangul(x))
        .collect()
}

/// The last word of a sentence, without punctuation or trailing non-Hangul.
pub fn last_word(sentence: &str) -> String {
    let stripped = TRAILING_MARKS.replace(sentence, "");
    let word = stripped.split_whitespace().last().unwrap_or("");
    TRAILING_NON_HANGUL.replace(word, "").into_owned()
}
