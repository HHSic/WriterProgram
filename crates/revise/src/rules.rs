//! Style rules for the revise mode. Prototype of the v0.2 rule set in docs/revise-engine.md.

use std::cmp::Reverse;
use std::collections::HashMap;

use lindera::LinderaResult;

use crate::hangul;
use crate::morph::{Analyzer, Morph, Tok};
use crate::text::{self, Kind, Paragraph};

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
        Self { rule, severity, para, title, hint: hint.into(), ranges, points: Vec::new(), fix: None }
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
        Self { genre: Genre::Print, long_sentence: 90, ending_run: 4, dialogue_run: 5, ..Self::web_novel() }
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
            sents.push(Sent { para: s.para, start: s.start, end: s.end, kind: s.kind, chars: slice.chars().count(), toks });
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
    out.sort_by_key(|i| (i.para, Reverse(i.severity), i.ranges.first().map_or(0, |r| r.0)));
    out
}

// ---------- helpers ----------

/// Byte range of the whitespace-delimited word containing `at`.
fn word_at(text: &str, at: usize) -> (usize, usize) {
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

fn first_word(doc: &Doc, s: &Sent) -> (usize, usize) {
    let (a, b) = word_at(doc.text, s.start);
    (a.max(s.start), b.min(s.end))
}

fn last_word(doc: &Doc, s: &Sent) -> (usize, usize) {
    let last = doc.text[s.start..s.end].char_indices().last().map_or(s.start, |(i, _)| s.start + i);
    let (a, b) = word_at(doc.text, last);
    (a.max(s.start), b.min(s.end))
}

fn trim_punct(s: &str) -> &str {
    s.trim_matches(|c: char| !c.is_alphanumeric())
}

/// Runs of consecutive narration sentences; dialogue breaks a run.
fn narration_runs(doc: &Doc) -> Vec<Vec<usize>> {
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
fn groups<K: PartialEq>(run: &[usize], key: impl Fn(usize) -> Option<K>) -> Vec<(K, Vec<usize>)> {
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

// ---------- A. repetition ----------

/// A1: consecutive sentences starting with the same word.
fn sentence_starts(doc: &Doc, p: &Profile, out: &mut Vec<Issue>) {
    for run in narration_runs(doc) {
        let key = |i: usize| -> Option<(String, bool)> {
            let m = doc.sents[i].toks.first()?.morphs.first()?;
            matches!(m.pos.as_str(), "NNG" | "NNP" | "NP" | "MAJ" | "MAG" | "MM")
                .then(|| (m.form.clone(), m.pos == "NP"))
        };
        for ((_, pronoun), group) in groups(&run, key) {
            let n = group.len();
            let severity = if n >= p.start_run {
                Severity::Mid
            } else if pronoun && n >= 2 {
                Severity::Low
            } else {
                continue;
            };
            let ranges: Vec<_> = group.iter().map(|&i| first_word(doc, &doc.sents[i])).collect();
            let word = trim_punct(doc.slice(ranges[0])).to_string();
            let hint = if pronoun {
                "한 번은 주어를 빼거나 이름·호칭으로 바꿔 보기"
            } else {
                "한 번은 주어를 빼거나 앞 문장과 합쳐 보기"
            };
            out.push(Issue::new(
                "A1",
                severity,
                doc.sents[group[0]].para,
                format!("‘{word}’{} 시작하는 문장이 {n}번", hangul::euro(&word)),
                hint,
                ranges,
            ));
        }
    }
}

fn ending_pattern(s: &Sent) -> Option<String> {
    let morphs: Vec<&Morph> = s.toks.iter().flat_map(|t| t.morphs.iter()).collect();
    let ef = morphs.iter().rposition(|m| m.pos == "EF")?;
    let mut k = ef;
    let mut eps = Vec::new();
    while k > 0 && morphs[k - 1].pos == "EP" {
        k -= 1;
        eps.push(match morphs[k].form.as_str() {
            "았" | "었" | "였" | "ㅆ" => "었".to_string(),
            f => f.to_string(),
        });
    }
    eps.reverse();
    Some(format!("{}{}", eps.concat(), morphs[ef].form))
}

/// A2: consecutive sentences with the same ending.
fn endings(doc: &Doc, p: &Profile, out: &mut Vec<Issue>) {
    for run in narration_runs(doc) {
        for (pattern, group) in groups(&run, |i| ending_pattern(&doc.sents[i])) {
            let n = group.len();
            if n < p.ending_run {
                continue;
            }
            let ranges = group.iter().map(|&i| last_word(doc, &doc.sents[i])).collect();
            out.push(Issue::new(
                "A2",
                Severity::Mid,
                doc.sents[group[0]].para,
                format!("‘-{pattern}.’{} 끝나는 문장이 {n}번 이어짐", hangul::euro(&pattern)),
                "한 문장은 현재형이나 명사형으로 끝내 보기",
                ranges,
            ));
        }
    }
}

const NOUN_STOP: &[&str] = &["것", "수", "때", "곳", "앞", "뒤", "안", "위", "속", "쪽", "중", "번", "듯", "채", "줄", "말", "일"];
const VERB_STOP: &[&str] = &["있", "하", "되", "없", "같", "않", "보", "오", "가", "알", "주", "들", "나", "모르", "싶", "이", "아니"];

/// A3: the same content word several times within a few words.
fn echoes(doc: &Doc, names: &Names, p: &Profile, out: &mut Vec<Issue>) {
    struct Occ {
        word: usize,
        range: (usize, usize),
        para: usize,
    }
    let mut word_starts = Vec::new();
    let mut prev_ws = true;
    for (i, c) in doc.text.char_indices() {
        if c.is_whitespace() {
            prev_ws = true;
        } else {
            if prev_ws {
                word_starts.push(i);
            }
            prev_ws = false;
        }
    }
    let word_index = |pos: usize| word_starts.partition_point(|&s| s <= pos).saturating_sub(1);

    let mut by_lemma: HashMap<String, Vec<Occ>> = HashMap::new();
    for s in &doc.sents {
        for t in &s.toks {
            for m in &t.morphs {
                let lemma = match m.pos.as_str() {
                    "NNG" | "NNP"
                        if m.form.chars().count() >= 2
                            && !NOUN_STOP.contains(&m.form.as_str())
                            && !names.contains(&m.form) =>
                    {
                        m.form.clone()
                    }
                    "VV" | "VA" if !m.form.is_empty() && !VERB_STOP.contains(&m.form.as_str()) => {
                        format!("{}다", m.form)
                    }
                    _ => continue,
                };
                by_lemma.entry(lemma).or_default().push(Occ { word: word_index(t.start), range: (t.start, t.end), para: s.para });
            }
        }
    }

    let mut lemmas: Vec<(String, Vec<Occ>)> = by_lemma.into_iter().collect();
    lemmas.sort_by_key(|(_, occs)| occs[0].range.0);
    for (lemma, occs) in lemmas {
        let mut i = 0;
        while i < occs.len() {
            let mut j = i;
            while j + 1 < occs.len() && occs[j + 1].word - occs[i].word < p.echo_window {
                j += 1;
            }
            let n = j - i + 1;
            if n >= p.echo_count {
                out.push(Issue::new(
                    "A3",
                    Severity::Mid,
                    occs[i].para,
                    format!("‘{lemma}’{} 가까이에서 {n}번 쓰임", hangul::i_ga(&lemma)),
                    "한 번은 대명사로 바꾸거나 빼 보기",
                    occs[i..=j].iter().map(|o| o.range).collect(),
                ));
                i = j + 1;
            } else {
                i += 1;
            }
        }
    }
}

// ---------- B. length and rhythm ----------

const CUT_EC: &[&str] = &[
    "는데", "은데", "ㄴ데", "ᆫ데", "던데", "고", "며", "으며", "면서", "으면서", "지만", "는데도", "어서", "아서", "여서", "으니", "니",
];

/// B1: long sentences, with the places where they could be cut.
fn long_sentences(doc: &Doc, p: &Profile, out: &mut Vec<Issue>) {
    for s in doc.sents.iter().filter(|s| s.kind == Kind::Narration) {
        let points: Vec<(usize, usize)> = s
            .toks
            .iter()
            .filter(|t| {
                t.morphs.last().is_some_and(|m| m.pos == "EC" && CUT_EC.contains(&m.form.as_str()))
                    && doc.text[t.end..].starts_with(char::is_whitespace)
            })
            .map(|t| word_at(doc.text, t.start))
            .collect();
        if s.chars <= p.long_sentence && points.len() < p.long_clauses {
            continue;
        }
        let hint = if points.is_empty() {
            "두세 문장으로 나눠 보기".to_string()
        } else {
            format!("표시한 {}곳에서 끊어 보기", points.len())
        };
        let mut issue =
            Issue::new("B1", Severity::Mid, s.para, format!("긴 문장 · {}자", s.chars), hint, vec![(s.start, s.end)]);
        issue.points = points;
        out.push(issue);
    }
}

/// B2: several sentences in a row with nearly the same length.
fn rhythm(doc: &Doc, p: &Profile, out: &mut Vec<Issue>) {
    for run in narration_runs(doc) {
        if run.len() < p.rhythm_sentences {
            continue;
        }
        let lens: Vec<f64> = run.iter().map(|&i| doc.sents[i].chars as f64).collect();
        let mut flagged = vec![false; run.len()];
        for w in 0..=run.len() - p.rhythm_sentences {
            let win = &lens[w..w + p.rhythm_sentences];
            let mean = win.iter().sum::<f64>() / win.len() as f64;
            let var = win.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / win.len() as f64;
            if mean >= 8.0 && var.sqrt() / mean < p.rhythm_cv {
                flagged[w..w + p.rhythm_sentences].iter_mut().for_each(|f| *f = true);
            }
        }
        let mut i = 0;
        while i < run.len() {
            if !flagged[i] {
                i += 1;
                continue;
            }
            let mut j = i;
            while j + 1 < run.len() && flagged[j + 1] {
                j += 1;
            }
            let (a, b) = (&doc.sents[run[i]], &doc.sents[run[j]]);
            out.push(Issue::new(
                "B2",
                Severity::Low,
                a.para,
                format!("문장 길이가 너무 고름 · {}문장", j - i + 1),
                "짧은 문장 하나를 섞거나 두 문장을 합쳐 보기",
                vec![(a.start, b.end)],
            ));
            i = j + 1;
        }
    }
}

/// B3: paragraphs too long for a phone (web novel), or many one-sentence paragraphs (print).
fn dense_paragraphs(doc: &Doc, p: &Profile, out: &mut Vec<Issue>) {
    match p.genre {
        Genre::WebNovel => {
            for para in doc.paras.iter().filter(|x| !x.excluded) {
                let lines = doc.text[para.start..para.end].chars().count().div_ceil(p.mobile_line_chars);
                if lines > p.mobile_max_lines {
                    out.push(Issue::new(
                        "B3",
                        Severity::Low,
                        para.index,
                        format!("휴대폰에서 {lines}줄인 문단"),
                        "문단을 나눠 보기",
                        vec![(para.start, para.end)],
                    ));
                }
            }
        }
        Genre::Print => {
            let mut run: Vec<usize> = Vec::new();
            let flush = |run: &mut Vec<usize>, out: &mut Vec<Issue>| {
                if run.len() >= p.one_sentence_run {
                    out.push(Issue::new(
                        "B3",
                        Severity::Low,
                        run[0],
                        format!("한 문장짜리 문단이 {}개 이어짐", run.len()),
                        "몇 개는 합쳐 보기",
                        run.iter().map(|&i| (doc.paras[i].start, doc.paras[i].end)).collect(),
                    ));
                }
                run.clear();
            };
            for para in doc.paras.iter().filter(|x| !x.excluded) {
                let sents: Vec<&Sent> = doc.sents.iter().filter(|s| s.para == para.index).collect();
                if sents.len() == 1 && sents[0].kind == Kind::Narration {
                    run.push(para.index);
                } else {
                    flush(&mut run, out);
                }
            }
            flush(&mut run, out);
        }
    }
}

// ---------- C. expression ----------

/// C1: many genitive particles '의' in one sentence.
fn genitive(doc: &Doc, out: &mut Vec<Issue>) {
    for s in doc.sents.iter().filter(|s| s.kind == Kind::Narration) {
        let ranges: Vec<_> = s.toks.iter().filter(|t| t.has_pos("JKG")).map(|t| (t.start, t.end)).collect();
        if ranges.len() >= 3 {
            out.push(Issue::new(
                "C1",
                Severity::Mid,
                s.para,
                format!("한 문장에 ‘의’가 {}번", ranges.len()),
                "‘A의 B의’처럼 이어지면 ‘의’ 하나를 빼 보기",
                ranges,
            ));
        }
    }
}

/// (passive stem, contracted form + 어): 보이 → 보여, so 보여지다 is a double passive.
const DOUBLE_PASSIVE: &[(&str, &str)] = &[
    ("보이", "보여"),
    ("쓰이", "쓰여"),
    ("놓이", "놓여"),
    ("모이", "모여"),
    ("쌓이", "쌓여"),
    ("잊히", "잊혀"),
    ("읽히", "읽혀"),
    ("닫히", "닫혀"),
    ("갇히", "갇혀"),
    ("먹히", "먹혀"),
    ("잡히", "잡혀"),
    ("불리", "불려"),
    ("열리", "열려"),
    ("풀리", "풀려"),
    ("팔리", "팔려"),
    ("걸리", "걸려"),
    ("믿기", "믿겨"),
    ("되", "되어"),
];

/// C2: double passives, with a safe replacement.
fn double_passive(doc: &Doc, out: &mut Vec<Issue>) {
    for s in &doc.sents {
        let body = &doc.text[s.start..s.end];
        for &(stem, contracted) in DOUBLE_PASSIVE {
            let mut from = 0;
            while let Some(found) = body[from..].find(contracted) {
                let at = from + found;
                let after = at + contracted.len();
                from = after;
                let Some(next) = body[after..].chars().next() else { continue };
                let replacement = match next {
                    '지' => stem.to_string(),
                    '졌' => hangul::with_last_final(contracted, hangul::JONG_SSANGSIOS),
                    '져' => contracted.to_string(),
                    '진' => hangul::with_last_final(stem, hangul::JONG_NIEUN),
                    '질' => hangul::with_last_final(stem, hangul::JONG_RIEUL),
                    _ => continue,
                };
                let (start, end) = (s.start + at, s.start + after + next.len_utf8());
                let word = word_at(doc.text, start);
                let fixed = format!("{}{}{}", &doc.text[word.0..start], replacement, &doc.text[end..word.1]);
                let shown = trim_punct(doc.slice(word));
                let fixed_shown = trim_punct(&fixed);
                let mut issue = Issue::new(
                    "C2",
                    Severity::Mid,
                    s.para,
                    format!("‘{shown}’{} 겹친 표현", hangul::eun_neun(shown)),
                    format!("‘{fixed_shown}’{} 자연스러움", hangul::i_ga(fixed_shown)),
                    vec![(start, end)],
                );
                issue.fix = Some(Fix { start, end, replacement });
                out.push(issue);
            }
        }
    }
}

const TRANSLATIONESE: &[(&str, &str)] = &[
    ("으로 인해", "‘~때문에’로 바꿔 보기"),
    ("로 인해", "‘~때문에’로 바꿔 보기"),
    ("에 있어서", "‘~에서’로 바꿔 보기"),
    ("에 대해", "‘~을/를’로 바꾸거나 문장을 다듬어 보기"),
    ("에 대한", "문장을 다듬어 보기"),
    ("를 통해", "‘~로’나 ‘~해서’로 바꿔 보기"),
    ("을 통해", "‘~로’나 ‘~해서’로 바꿔 보기"),
    ("에 의해", "능동문으로 바꿔 보기"),
    ("하고 있는 중", "‘~하는 중’이나 ‘~하고 있다’로 줄여 보기"),
    ("를 가지고 있", "‘~이 있다’로 바꿔 보기"),
    ("을 가지고 있", "‘~이 있다’로 바꿔 보기"),
];

/// C3: translated-sounding phrases. Summary only.
fn translationese(doc: &Doc, out: &mut Vec<Issue>) {
    for s in doc.sents.iter().filter(|s| s.kind == Kind::Narration) {
        let body = &doc.text[s.start..s.end];
        let mut taken: Vec<(usize, usize)> = Vec::new();
        for &(phrase, hint) in TRANSLATIONESE {
            for (at, _) in body.match_indices(phrase) {
                let r = (s.start + at, s.start + at + phrase.len());
                if taken.iter().any(|t| t.0 < r.1 && r.0 < t.1) {
                    continue;
                }
                taken.push(r);
                out.push(Issue::new("C3", Severity::Low, s.para, format!("번역 투 표현 · ‘~{phrase}’"), hint, vec![r]));
            }
        }
    }
}

// ---------- D. dialogue ----------

/// D1: many dialogue-only paragraphs in a row.
fn dialogue_runs(doc: &Doc, p: &Profile, out: &mut Vec<Issue>) {
    let mut run: Vec<usize> = Vec::new();
    let flush = |run: &mut Vec<usize>, out: &mut Vec<Issue>| {
        if run.len() >= p.dialogue_run {
            out.push(Issue::new(
                "D1",
                Severity::Mid,
                run[0],
                format!("대사만 {}줄 이어짐 · 누가 말하는지 헷갈릴 수 있음", run.len()),
                "중간에 행동 묘사나 ‘누가 말했다’를 넣어 보기",
                run.iter().map(|&i| (doc.paras[i].start, doc.paras[i].end)).collect(),
            ));
        }
        run.clear();
    };
    for para in doc.paras.iter().filter(|x| !x.excluded) {
        let mut kinds = doc.sents.iter().filter(|s| s.para == para.index).map(|s| s.kind).peekable();
        let has_any = kinds.peek().is_some();
        if has_any && kinds.all(|k| k != Kind::Narration) {
            run.push(para.index);
        } else {
            flush(&mut run, out);
        }
    }
    flush(&mut run, out);
}

// ---------- E. setting cards ----------

const PARTICLES: &[&str] = &[
    "", "는", "은", "이", "가", "을", "를", "의", "에게", "한테", "와", "과", "도", "만", "야", "아", "씨", "님", "께", "에게서",
    "이는", "이가", "이를", "이의", "이도", "이와", "이만", "이야", "이에게", "이한테",
];

fn split_words(s: &str) -> Vec<(usize, &str)> {
    let mut words = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in s.char_indices() {
        if c.is_whitespace() {
            if let Some(st) = start.take() {
                words.push((st, &s[st..i]));
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(st) = start {
        words.push((st, &s[st..]));
    }
    words
}

/// E1: a name one jamo away from a setting-card name, used rarely, while the card name is common.
fn name_typos(doc: &Doc, names: &Names, out: &mut Vec<Issue>) {
    let targets: Vec<&str> =
        names.all().filter(|n| !n.contains(char::is_whitespace) && n.chars().count() >= 2).collect();
    if targets.is_empty() {
        return;
    }
    let count = |w: &str| doc.sents.iter().map(|s| doc.text[s.start..s.end].matches(w).count()).sum::<usize>();
    for s in &doc.sents {
        let body = &doc.text[s.start..s.end];
        for (off, raw) in split_words(body) {
            let word = trim_punct(raw);
            if word.is_empty() {
                continue;
            }
            let lead = raw.find(word).unwrap_or(0);
            let chars: Vec<char> = word.chars().collect();
            for &name in &targets {
                let len = name.chars().count();
                if chars.len() < len {
                    continue;
                }
                let cand: String = chars[..len].iter().collect();
                let rest: String = chars[len..].iter().collect();
                if !PARTICLES.contains(&rest.as_str())
                    || names.contains(&cand)
                    || !cand.chars().all(hangul::is_syllable)
                    || hangul::edit_distance(&hangul::jamo(&cand), &hangul::jamo(name)) != 1
                    || count(&cand) > 2
                    || count(name) < 3
                {
                    continue;
                }
                let start = s.start + off + lead;
                let end = start + cand.len();
                let mut issue = Issue::new(
                    "E1",
                    Severity::Mid,
                    s.para,
                    format!("‘{cand}’{} 설정집에 없는 이름", hangul::eun_neun(&cand)),
                    format!("‘{name}’{} 잘못 쓴 것일 수 있음", hangul::eul_reul(name)),
                    vec![(start, end)],
                );
                issue.fix = Some(Fix { start, end, replacement: name.to_string() });
                out.push(issue);
                break;
            }
        }
    }
}
