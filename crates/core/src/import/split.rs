//! Cutting a file's items into chapters: title lines, heading styles, and
//! the rule that fits a file best.

use std::sync::LazyLock;

use regex::Regex;

use super::{Chapter, ImportOptions, Item, Skip, SkipKind, SplitRule};
use crate::markup::{Block, Inline, inline_text};

const AFTER_NUMBER: &str = r"(?:$|[\s.:·\-—)\]])";

static EPISODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^(?:제\s*)?\d+\s*[화회]{AFTER_NUMBER}")).unwrap());
static CHAPTER: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(&format!(
        r"^(?:(?:제\s*)?\d+\s*장|(?i:chapter|ch)\.?\s*\d+){AFTER_NUMBER}"
    ))
    .unwrap()
});
static NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(&format!(r"^#\s*\d+{AFTER_NUMBER}")).unwrap());
/// The number in front of a title, which the app numbers by itself.
static NUMBER_PREFIX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?:(?:제\s*)?\d+\s*[화회장]|(?i:chapter|ch)\.?\s*\d+|#\s*\d+)").unwrap()
});

/// Longest line that can be a chapter title.
const TITLE_MAX_CHARS: usize = 60;
const TITLE_KEEP_CHARS: usize = 100;

fn is_separator(c: char) -> bool {
    c.is_whitespace() || matches!(c, '.' | ':' | '·' | '-' | '—' | ')' | ']')
}

/// "제3화 비에 젖은 손님" is titled "비에 젖은 손님": chapters are numbered by
/// their place in the tree. A line with nothing but the number keeps it.
fn clean_title(line: &str) -> String {
    let line = line.trim();
    let rest = NUMBER_PREFIX
        .find(line)
        .map(|m| line[m.end()..].trim_start_matches(is_separator).trim_end())
        .filter(|rest| !rest.is_empty())
        .unwrap_or(line);
    rest.chars().take(TITLE_KEEP_CHARS).collect()
}

/// Ways of telling a chapter's title line.
#[derive(Clone)]
pub(super) enum Split {
    File,
    Heading(u8),
    Pattern { re: Regex, custom: bool },
}

impl Split {
    pub(super) fn label(&self, rule: SplitRule) -> &'static str {
        match (self, rule) {
            (Split::File, _) => "파일 하나가 회차 하나",
            (Split::Heading(_), _) => "제목 서식",
            (Split::Pattern { custom: true, .. }, _) => "직접 쓴 패턴",
            (Split::Pattern { re, .. }, _) if re.as_str() == EPISODE.as_str() => "제N화",
            (Split::Pattern { re, .. }, _) if re.as_str() == CHAPTER.as_str() => {
                "제N장 · Chapter N"
            }
            _ => "#N",
        }
    }
}

/// The first line of a paragraph and what follows it.
fn split_first_line(inlines: &[Inline]) -> (String, Vec<Inline>) {
    let cut = inlines
        .iter()
        .position(|i| matches!(i, Inline::HardBreak {}))
        .unwrap_or(inlines.len());
    let rest = inlines
        .get(cut + 1..)
        .map(<[Inline]>::to_vec)
        .unwrap_or_default();
    (inline_text(&inlines[..cut]), rest)
}

fn is_title_line(re: &Regex, line: &str) -> bool {
    let line = line.trim();
    line.chars().count() <= TITLE_MAX_CHARS && re.is_match(line)
}

/// Headings become plain paragraphs, for rules that do not cut at them.
fn demote(item: Item) -> Item {
    match item {
        Item::Heading { text, .. } => Item::Para(vec![Inline::Text {
            text,
            marks: vec![],
        }]),
        other => other,
    }
}

/// How many title lines `re` finds, and whether the first thing in the file is one.
fn count_pattern(items: &[Item], re: &Regex) -> (usize, bool) {
    let mut count = 0;
    let mut first = None;
    for item in items {
        let line = match item {
            Item::Para(inl) => split_first_line(inl).0,
            Item::Heading { text, .. } => text.clone(),
            _ => continue,
        };
        let hit = is_title_line(re, &line);
        first.get_or_insert(hit);
        count += usize::from(hit);
    }
    (count, first == Some(true))
}

fn fits(count: usize, first: bool) -> bool {
    count >= 2 || (count == 1 && first)
}

pub(super) fn resolve(items: &[Item], opts: &ImportOptions) -> std::result::Result<Split, String> {
    let heading = |items: &[Item]| {
        let min = items
            .iter()
            .filter_map(|i| match i {
                Item::Heading { level, .. } => Some(*level),
                _ => None,
            })
            .min()?;
        let count = items
            .iter()
            .filter(|i| matches!(i, Item::Heading { level, .. } if *level == min))
            .count();
        let first = items
            .iter()
            .find(|i| !matches!(i, Item::Skip(..)))
            .is_some_and(|i| matches!(i, Item::Heading { level, .. } if *level == min));
        Some((min, count, first))
    };
    let pattern = |re: &LazyLock<Regex>| Split::Pattern {
        re: Regex::clone(re),
        custom: false,
    };
    Ok(match opts.rule {
        SplitRule::File => Split::File,
        SplitRule::Heading => heading(items).map_or(Split::File, |(min, ..)| Split::Heading(min)),
        SplitRule::Episode => pattern(&EPISODE),
        SplitRule::Chapter => pattern(&CHAPTER),
        SplitRule::Number => pattern(&NUMBER),
        SplitRule::Regex => {
            let re = Regex::new(opts.pattern.trim())
                .map_err(|_| "직접 쓴 패턴이 올바르지 않음".to_string())?;
            if opts.pattern.trim().is_empty() {
                return Err("직접 쓴 패턴이 비어 있음".into());
            }
            Split::Pattern { re, custom: true }
        }
        SplitRule::Auto => {
            if let Some((min, ..)) = heading(items).filter(|(_, c, f)| fits(*c, *f)) {
                return Ok(Split::Heading(min));
            }
            [&EPISODE, &CHAPTER, &NUMBER]
                .into_iter()
                .find(|re| {
                    let (count, first) = count_pattern(items, re);
                    fits(count, first)
                })
                .map_or(Split::File, pattern)
        }
    })
}

fn push_skip(skips: &mut Vec<Skip>, at: usize, kind: SkipKind, count: u32) {
    match skips.last_mut() {
        Some(last) if last.at == at && last.kind == kind => last.count += count,
        _ => skips.push(Skip { at, kind, count }),
    }
}

/// Cuts a file's items into chapters. The part before the first title, if
/// there is one, is a chapter named after the file.
pub(super) fn split(items: Vec<Item>, how: &Split, stem: &str, file: usize) -> Vec<Chapter> {
    let items: Vec<Item> = match how {
        Split::File => items.into_iter().map(demote).collect(),
        Split::Heading(level) => items
            .into_iter()
            .map(|i| match i {
                Item::Heading { level: l, .. } if l != *level => demote(i),
                other => other,
            })
            .collect(),
        Split::Pattern { re, .. } => {
            let mut out = Vec::with_capacity(items.len());
            for item in items.into_iter().map(demote) {
                match item {
                    Item::Para(inl) => {
                        let (line, rest) = split_first_line(&inl);
                        if is_title_line(re, &line) {
                            out.push(Item::Heading {
                                level: 1,
                                text: line.trim().to_string(),
                            });
                            if !inline_text(&rest).trim().is_empty() {
                                out.push(Item::Para(rest));
                            }
                        } else {
                            out.push(Item::Para(inl));
                        }
                    }
                    other => out.push(other),
                }
            }
            out
        }
    };

    let new = |title: String| Chapter {
        file,
        title,
        body: Vec::new(),
        skips: Vec::new(),
    };
    let mut lead = new(stem.chars().take(TITLE_KEEP_CHARS).collect());
    let mut titled: Vec<Chapter> = Vec::new();
    for item in items {
        if let Item::Heading { text, .. } = &item {
            let title = match how {
                Split::Pattern { re, custom: true } => re
                    .captures(text)
                    .and_then(|c| c.get(1))
                    .map_or(text.trim(), |m| m.as_str().trim())
                    .chars()
                    .take(TITLE_KEEP_CHARS)
                    .collect(),
                _ => clean_title(text),
            };
            titled.push(new(title));
            continue;
        }
        let chapter = titled.last_mut().unwrap_or(&mut lead);
        match item {
            Item::Para(inl) => chapter.body.push(Block::para(inl)),
            Item::Scene => chapter.body.push(Block::SceneBreak {}),
            Item::Skip(kind, count) => {
                let at = chapter.body.len();
                push_skip(&mut chapter.skips, at, kind, count);
            }
            Item::Heading { .. } => unreachable!("handled above"),
        }
    }
    if titled.is_empty() {
        if lead.body.is_empty() && lead.skips.is_empty() {
            return Vec::new();
        }
        return vec![lead];
    }
    if !lead.body.is_empty() || !lead.skips.is_empty() {
        titled.insert(0, lead);
    }
    titled
}
