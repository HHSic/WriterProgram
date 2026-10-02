//! Style rules for the revise mode. Prototype of the v0.2 rule set in docs/revise-engine.md.

mod dialogue;
mod expression;
mod helpers;
mod names;
mod repetition;
mod rhythm;

use std::cmp::Reverse;

use lindera::LinderaResult;

use crate::morph::{Analyzer, Tok};
use crate::text::{self, Kind, Paragraph};
use dialogue::dialogue_runs;
use expression::{double_passive, genitive, translationese};
use names::name_typos;
use repetition::{echoes, endings, sentence_starts};
use rhythm::{dense_paragraphs, long_sentences, rhythm};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Severity {
    /// Summary only, not decorated in the text.
    Low,
    /// Decorated in the text with a margin card.
    Mid,
    High,
}

impl Severity {
    pub fn in_text(self) -> bool {
        self >= Severity::Mid
    }
}

#[derive(Clone, Debug)]
pub struct Fix {
    pub start: usize,
    pub end: usize,
    pub replacement: String,
}

#[derive(Clone, Debug)]
pub struct Issue {
    pub rule: &'static str,
    pub severity: Severity,
    pub para: usize,
    pub title: String,
    pub hint: String,
    /// Spans to decorate.
    pub ranges: Vec<(usize, usize)>,
    /// Secondary marks, e.g. where a long sentence could be cut.
    pub points: Vec<(usize, usize)>,
    /// Safe one-click replacement.
    pub fix: Option<Fix>,
}

impl Issue {
    fn new(
        rule: &'static str,
        severity: Severity,
        para: usize,
        title: String,
        hint: impl Into<String>,
        ranges: Vec<(usize, usize)>,
    ) -> Self {
        Self {
            rule,
            severity,
            para,
            title,
            hint: hint.into(),
            ranges,
            points: Vec::new(),
            fix: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Genre {
    WebNovel,
    Print,
}

#[derive(Clone, Debug)]
pub struct Profile {
    pub genre: Genre,
    /// B1: characters (spaces included).
    pub long_sentence: usize,
    /// B1: number of cut points (connective endings) that makes a sentence long.
    pub long_clauses: usize,
    /// A1
    pub start_run: usize,
    /// A2
    pub ending_run: usize,
    /// A3: window in words, and hits inside it.
    pub echo_window: usize,
    pub echo_count: usize,
    /// B2
    pub rhythm_sentences: usize,
    pub rhythm_cv: f64,
    /// D1: consecutive dialogue-only paragraphs.
    pub dialogue_run: usize,
    /// B3 (web novel): characters per line on a 360px phone, and the line limit.
    pub mobile_line_chars: usize,
    pub mobile_max_lines: usize,
    /// B3 (print): consecutive one-sentence paragraphs.
    pub one_sentence_run: usize,
    /// Noise control: text decorations per paragraph.
    pub per_paragraph_cap: usize,
}

impl Profile {
    pub fn web_novel() -> Self {
        Self {
            genre: Genre::WebNovel,
            long_sentence: 70,
            long_clauses: 4,
            start_run: 3,
            ending_run: 5,
            echo_window: 30,
            echo_count: 3,
            rhythm_sentences: 6,
            rhythm_cv: 0.15,
            dialogue_run: 6,
            mobile_line_chars: 19,
            mobile_max_lines: 7,
            one_sentence_run: 5,
            per_paragraph_cap: 2,
        }
    }

    pub fn print() -> Self {
        Self {
            genre: Genre::Print,
            long_sentence: 90,
            ending_run: 4,
            dialogue_run: 5,
            ..Self::web_novel()
        }
    }

    pub fn label(&self) -> &'static str {
        match self.genre {
            Genre::WebNovel => "웹소설",
            Genre::Print => "출판 장편",
        }
    }
}

/// Setting-card names. One line per card: `이름,다른 이름,다른 이름`.
#[derive(Clone, Debug, Default)]
pub struct Names {
    entries: Vec<Vec<String>>,
}

impl Names {
    pub fn parse(src: &str) -> Self {
        let entries = src
            .lines()
            .map(|line| {
                line.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .filter(|entry| !entry.is_empty())
            .collect();
        Self { entries }
    }

    pub fn all(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().flatten().map(String::as_str)
    }

    pub fn contains(&self, word: &str) -> bool {
        self.all().any(|n| n == word)
    }

    /// ko-dic user dictionary lines (`surface,NNP,reading`) so names stay one token.
    pub fn user_dictionary_csv(&self) -> String {
        self.all()
            .filter(|n| !n.contains(char::is_whitespace))
            .map(|n| format!("{n},NNP,{n}\n"))
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct Sent {
    pub para: usize,
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
    pub chars: usize,
    pub toks: Vec<Tok>,
}

pub struct Doc<'a> {
    pub text: &'a str,
    pub paras: Vec<Paragraph>,
    pub sents: Vec<Sent>,
}

impl<'a> Doc<'a> {
    fn slice(&self, r: (usize, usize)) -> &'a str {
        &self.text[r.0..r.1]
    }
}

pub fn analyze<'a>(text: &'a str, analyzer: &Analyzer) -> LinderaResult<Doc<'a>> {
    let paras = text::paragraphs(text);
    let mut sents = Vec::new();
    for p in &paras {
        for s in text::sentences(text, p) {
            let slice = &text[s.start..s.end];
            let toks = analyzer.analyze(slice, s.start)?;
            sents.push(Sent {
                para: s.para,
                start: s.start,
                end: s.end,
                kind: s.kind,
                chars: slice.chars().count(),
                toks,
            });
        }
    }
    Ok(Doc { text, paras, sents })
}

pub fn check(doc: &Doc, names: &Names, p: &Profile) -> Vec<Issue> {
    let mut out = Vec::new();
    sentence_starts(doc, p, &mut out);
    endings(doc, p, &mut out);
    echoes(doc, names, p, &mut out);
    long_sentences(doc, p, &mut out);
    rhythm(doc, p, &mut out);
    dense_paragraphs(doc, p, &mut out);
    genitive(doc, &mut out);
    double_passive(doc, &mut out);
    translationese(doc, &mut out);
    dialogue_runs(doc, p, &mut out);
    name_typos(doc, names, &mut out);
    out.sort_by_key(|i| {
        (
            i.para,
            Reverse(i.severity),
            i.ranges.first().map_or(0, |r| r.0),
        )
    });
    out
}
