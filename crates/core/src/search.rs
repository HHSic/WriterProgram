//! Find and replace across the project (작품 전체 찾기/바꾸기, S9).
//!
//! Matching runs on each paragraph's plain text, with line breaks as `\n`,
//! so a match never crosses paragraphs. Offsets sent to the editor are in
//! UTF-16 code units, the way ProseMirror counts text.

use std::path::{Path, PathBuf};

use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

use crate::doc::{self, DocFile};
use crate::markup::{Block, Inline, Mark, inline_text};
use crate::snapshot::{self, SnapshotInfo};
use crate::{Error, Result, project};

/// Most matches returned for one search; the screen says when there are more.
pub const MAX_MATCHES: usize = 2000;
const CONTEXT_CHARS: usize = 24;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchQuery {
    pub text: String,
    /// Treat `text` as a regular expression.
    #[serde(default)]
    pub regex: bool,
    /// Only whole words (not inside a longer word).
    #[serde(default)]
    pub whole_word: bool,
    /// Documents to search, in order. `None` searches the whole project
    /// (manuscript, then planning documents).
    #[serde(default)]
    pub doc_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Match {
    /// Index of the paragraph among the document's blocks.
    pub block: usize,
    /// UTF-16 offsets within the paragraph's text.
    pub start: usize,
    pub end: usize,
    pub before: String,
    pub text: String,
    pub after: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocMatches {
    pub doc_id: String,
    pub matches: Vec<Match>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub docs: Vec<DocMatches>,
    pub total: usize,
    /// More matches exist than were returned.
    pub truncated: bool,
    /// Documents in scope whose file is there but cannot be read (mend.rs),
    /// left out: "열 수 없는 회차 2개는 빼고 찾았습니다".
    pub skipped: usize,
}

pub fn matcher(q: &SearchQuery) -> Result<Regex> {
    if q.text.is_empty() {
        return Err(Error::Invalid("찾을 말을 적어 주세요".into()));
    }
    let pattern = if q.regex {
        q.text.clone()
    } else {
        regex::escape(&q.text)
    };
    let pattern = if q.whole_word {
        format!(r"(?:^|\b){pattern}(?:\b|$)")
    } else {
        pattern
    };
    RegexBuilder::new(&pattern)
        .size_limit(1 << 20)
        .build()
        .map_err(|_| Error::Invalid("찾는 식이 올바르지 않음".into()))
}

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

fn context_before(text: &str, byte: usize) -> String {
    let head = &text[..byte];
    let line_start = head.rfind('\n').map_or(0, |i| i + 1);
    let chars: Vec<char> = head[line_start..].chars().collect();
    let skip = chars.len().saturating_sub(CONTEXT_CHARS);
    let mut out: String = chars[skip..].iter().collect();
    if skip > 0 || line_start > 0 {
        out.insert(0, '…');
    }
    out
}

fn context_after(text: &str, byte: usize) -> String {
    let tail = &text[byte..];
    let line_end = tail.find('\n').unwrap_or(tail.len());
    let chars: Vec<char> = tail[..line_end].chars().collect();
    let mut out: String = chars.iter().take(CONTEXT_CHARS).collect();
    if chars.len() > CONTEXT_CHARS || line_end < tail.len() {
        out.push('…');
    }
    out
}

/// Matches in one document's blocks.
pub fn find_in_blocks(blocks: &[Block], re: &Regex, limit: usize) -> Vec<Match> {
    let mut out = Vec::new();
    for (block, b) in blocks.iter().enumerate() {
        let Block::Paragraph { content, .. } = b else {
            continue;
        };
        let text = inline_text(content);
        for m in re.find_iter(&text) {
            if m.is_empty() || m.as_str().contains('\n') {
                continue;
            }
            if out.len() >= limit {
                return out;
            }
            out.push(Match {
                block,
                start: utf16_len(&text[..m.start()]),
                end: utf16_len(&text[..m.end()]),
                before: context_before(&text, m.start()),
                text: m.as_str().to_string(),
                after: context_after(&text, m.end()),
            });
        }
    }
    out
}

fn scope(root: &Path, q: &SearchQuery) -> Result<Vec<String>> {
    match &q.doc_ids {
        Some(ids) => Ok(ids.clone()),
        None => Ok(project::doc_ids(&project::load(root)?)),
    }
}

pub fn search(root: &Path, q: &SearchQuery) -> Result<SearchResult> {
    let re = matcher(q)?;
    let mut docs = Vec::new();
    let mut total = 0;
    let mut truncated = false;
    let mut skipped = 0;
    for id in scope(root, q)? {
        let Some((_, file)) = readable(root, &id, &mut skipped) else {
            continue;
        };
        let left = MAX_MATCHES.saturating_sub(total);
        let matches = find_in_blocks(&file.body, &re, left + 1);
        if matches.len() > left {
            truncated = true;
        }
        let matches: Vec<Match> = matches.into_iter().take(left).collect();
        if !matches.is_empty() {
            total += matches.len();
            docs.push(DocMatches {
                doc_id: id,
                matches,
            });
        }
        if truncated {
            break;
        }
    }
    Ok(SearchResult {
        docs,
        total,
        truncated,
        skipped,
    })
}

/// A document in scope: none when its file is missing (it may be on its way
/// from another device) or cannot be read, which `skipped` counts.
fn readable(root: &Path, id: &str, skipped: &mut usize) -> Option<(PathBuf, DocFile)> {
    let (_, path) = doc::locate(root, id).ok()?;
    match doc::read_doc(&path) {
        Ok(file) => Some((path, file)),
        Err(_) => {
            *skipped += 1;
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Replacing

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplacedDoc {
    pub doc_id: String,
    pub count: usize,
    /// The "바꾸기 전" record kept before the change.
    pub snapshot: SnapshotInfo,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaceOutcome {
    pub replaced: usize,
    pub docs: Vec<ReplacedDoc>,
    /// Documents in scope that cannot be read, left as they are.
    pub skipped: usize,
}

#[derive(Clone)]
enum Atom {
    Char(char, Vec<Mark>),
    Break,
}

fn atoms(content: &[Inline]) -> Vec<Atom> {
    let mut out = Vec::new();
    for inline in content {
        match inline {
            Inline::Text { text, marks } => {
                out.extend(text.chars().map(|c| Atom::Char(c, marks.clone())))
            }
            Inline::HardBreak {} => out.push(Atom::Break),
        }
    }
    out
}

fn inlines(atoms: &[Atom]) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::new();
    for atom in atoms {
        match atom {
            Atom::Break => out.push(Inline::HardBreak {}),
            Atom::Char(c, marks) => match out.last_mut() {
                Some(Inline::Text { text, marks: m }) if m == marks => text.push(*c),
                _ => out.push(Inline::Text {
                    text: c.to_string(),
                    marks: marks.clone(),
                }),
            },
        }
    }
    out
}

/// Replaces every match in a paragraph. The new text takes the marks of the
/// first character it replaces. Returns the new content and the count.
fn replace_in_paragraph(
    content: &[Inline],
    re: &Regex,
    replacement: &str,
    expand: bool,
) -> (Vec<Inline>, usize) {
    let text = inline_text(content);
    let old = atoms(content);
    let mut new: Vec<Atom> = Vec::with_capacity(old.len());
    let mut count = 0;
    let mut char_pos = 0; // chars of `text` consumed into `new`
    let mut byte_pos = 0;
    for caps in re.captures_iter(&text) {
        let m = caps.get(0).expect("whole match");
        if m.is_empty() || m.as_str().contains('\n') {
            continue;
        }
        let start_char = char_pos + text[byte_pos..m.start()].chars().count();
        let len_chars = m.as_str().chars().count();
        new.extend_from_slice(&old[char_pos..start_char]);
        let marks = match &old[start_char] {
            Atom::Char(_, marks) => marks.clone(),
            Atom::Break => Vec::new(),
        };
        let mut with = String::new();
        if expand {
            caps.expand(replacement, &mut with);
        } else {
            with.push_str(replacement);
        }
        for c in with.chars() {
            new.push(if c == '\n' {
                Atom::Break
            } else {
                Atom::Char(c, marks.clone())
            });
        }
        char_pos = start_char + len_chars;
        byte_pos = m.end();
        count += 1;
    }
    if count == 0 {
        return (content.to_vec(), 0);
    }
    new.extend_from_slice(&old[char_pos..]);
    (inlines(&new), count)
}

pub fn replace_in_blocks(
    blocks: &[Block],
    re: &Regex,
    replacement: &str,
    expand: bool,
) -> (Vec<Block>, usize) {
    let mut total = 0;
    let out = blocks
        .iter()
        .map(|b| match b {
            Block::Paragraph { attrs, content } => {
                let (content, n) = replace_in_paragraph(content, re, replacement, expand);
                total += n;
                Block::Paragraph {
                    attrs: *attrs,
                    content,
                }
            }
            Block::SceneBreak {} => Block::SceneBreak {},
        })
        .collect();
    (out, total)
}

/// Replaces every match in scope. Each changed document first gets a
/// "before-replace" record, so the change can be taken back.
pub fn replace_all(root: &Path, q: &SearchQuery, replacement: &str) -> Result<ReplaceOutcome> {
    let re = matcher(q)?;
    let mut docs = Vec::new();
    let mut replaced = 0;
    let mut skipped = 0;
    for id in scope(root, q)? {
        let Some((path, file)) = readable(root, &id, &mut skipped) else {
            continue;
        };
        let (body, count) = replace_in_blocks(&file.body, &re, replacement, q.regex);
        if count == 0 {
            continue;
        }
        let snapshot = snapshot::create(root, &file, "before-replace", "")?;
        doc::write_doc_file(
            &path,
            &DocFile {
                meta: file.meta,
                body,
            },
        )?;
        replaced += count;
        docs.push(ReplacedDoc {
            doc_id: id,
            count,
            snapshot,
        });
    }
    Ok(ReplaceOutcome {
        replaced,
        docs,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markup::Block;

    fn q(text: &str) -> SearchQuery {
        SearchQuery {
            text: text.into(),
            regex: false,
            whole_word: false,
            doc_ids: None,
        }
    }

    #[test]
    fn finds_with_context_and_utf16_offsets() {
        let blocks = vec![
            Block::text("😀 서하는 우산을 꺼냈다. 서하가 웃었다."),
            Block::SceneBreak {},
            Block::text("첫 줄\n서하 둘째 줄"),
        ];
        let found = find_in_blocks(&blocks, &matcher(&q("서하")).unwrap(), 100);
        assert_eq!(found.len(), 3);
        // The emoji takes two UTF-16 units.
        assert_eq!((found[0].block, found[0].start, found[0].end), (0, 3, 5));
        assert_eq!(found[0].before, "😀 ");
        assert!(found[0].after.starts_with("는 우산을"));
        assert_eq!(found[2].block, 2);
        assert_eq!(found[2].before, "…");
    }

    #[test]
    fn whole_words_and_regex() {
        let blocks = vec![Block::text("서하 서하는 윤서하")];
        let mut query = q("서하");
        query.whole_word = true;
        assert_eq!(
            find_in_blocks(&blocks, &matcher(&query).unwrap(), 100).len(),
            1
        );
        let mut query = q("서하[는가]");
        query.regex = true;
        assert_eq!(
            find_in_blocks(&blocks, &matcher(&query).unwrap(), 100).len(),
            1
        );
        query.text = "(".into();
        assert!(matcher(&query).is_err());
    }

    #[test]
    fn replace_keeps_marks_and_breaks() {
        let content = vec![
            Inline::Text {
                text: "서하는 ".into(),
                marks: vec![],
            },
            Inline::Text {
                text: "서하".into(),
                marks: vec![Mark::Bold {}],
            },
            Inline::HardBreak {},
            Inline::Text {
                text: "서하".into(),
                marks: vec![],
            },
        ];
        let blocks = vec![Block::para(content)];
        let (out, n) = replace_in_blocks(&blocks, &matcher(&q("서하")).unwrap(), "윤서하", false);
        assert_eq!(n, 3);
        assert_eq!(
            out,
            vec![Block::Paragraph {
                attrs: Default::default(),
                content: vec![
                    Inline::Text {
                        text: "윤서하는 ".into(),
                        marks: vec![]
                    },
                    Inline::Text {
                        text: "윤서하".into(),
                        marks: vec![Mark::Bold {}]
                    },
                    Inline::HardBreak {},
                    Inline::Text {
                        text: "윤서하".into(),
                        marks: vec![]
                    },
                ]
            }]
        );
    }

    #[test]
    fn regex_replacement_expands_groups() {
        let blocks = vec![Block::text("12화 13화")];
        let mut query = q(r"(\d+)화");
        query.regex = true;
        let (out, n) = replace_in_blocks(&blocks, &matcher(&query).unwrap(), "제$1화", true);
        assert_eq!(n, 2);
        assert_eq!(out, vec![Block::text("제12화 제13화")]);
        // Without regex the replacement is literal.
        let (out, _) =
            replace_in_blocks(&[Block::text("a")], &matcher(&q("a")).unwrap(), "$1", false);
        assert_eq!(out, vec![Block::text("$1")]);
    }
}
