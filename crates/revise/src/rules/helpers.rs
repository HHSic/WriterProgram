//! Helpers the rules share: words around a position, runs of narration.

use super::{Doc, Sent};
use crate::text::Kind;

/// Byte range of the whitespace-delimited word containing `at`.
pub(super) fn word_at(text: &str, at: usize) -> (usize, usize) {
    let start = text[..at]
        .char_indices()
        .rev()
        .find(|(_, c)| c.is_whitespace())
        .map_or(0, |(i, c)| i + c.len_utf8());
    let end = text[at..]
        .char_indices()
        .find(|(_, c)| c.is_whitespace())
        .map_or(text.len(), |(i, _)| at + i);
    (start, end)
}

pub(super) fn first_word(doc: &Doc, s: &Sent) -> (usize, usize) {
    let (a, b) = word_at(doc.text, s.start);
    (a.max(s.start), b.min(s.end))
}

pub(super) fn last_word(doc: &Doc, s: &Sent) -> (usize, usize) {
    let last = doc.text[s.start..s.end]
        .char_indices()
        .last()
        .map_or(s.start, |(i, _)| s.start + i);
    let (a, b) = word_at(doc.text, last);
    (a.max(s.start), b.min(s.end))
}

pub(super) fn trim_punct(s: &str) -> &str {
    s.trim_matches(|c: char| !c.is_alphanumeric())
}

/// Runs of consecutive narration sentences; dialogue breaks a run.
pub(super) fn narration_runs(doc: &Doc) -> Vec<Vec<usize>> {
    let mut runs = Vec::new();
    let mut cur = Vec::new();
    for (i, s) in doc.sents.iter().enumerate() {
        if s.kind == Kind::Narration {
            cur.push(i);
        } else if !cur.is_empty() {
            runs.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        runs.push(cur);
    }
    runs
}

/// Groups consecutive items of `run` that share the same key.
pub(super) fn groups<K: PartialEq>(
    run: &[usize],
    key: impl Fn(usize) -> Option<K>,
) -> Vec<(K, Vec<usize>)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < run.len() {
        let mut j = i + 1;
        if let Some(k) = key(run[i]) {
            while j < run.len() && key(run[j]).as_ref() == Some(&k) {
                j += 1;
            }
            out.push((k, run[i..j].to_vec()));
        }
        i = j;
    }
    out
}
