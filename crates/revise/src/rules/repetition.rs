//! A. Repetition: sentence starts (A1), endings (A2), and the same word
//! close together (A3).

use std::collections::HashMap;

use super::helpers::{first_word, groups, last_word, narration_runs, trim_punct};
use super::{Doc, Issue, Names, Profile, Sent, Severity};
use crate::hangul;
use crate::morph::Morph;

/// A1: consecutive sentences starting with the same word.
pub(super) fn sentence_starts(doc: &Doc, p: &Profile, out: &mut Vec<Issue>) {
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
            let ranges: Vec<_> = group
                .iter()
                .map(|&i| first_word(doc, &doc.sents[i]))
                .collect();
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
pub(super) fn endings(doc: &Doc, p: &Profile, out: &mut Vec<Issue>) {
    for run in narration_runs(doc) {
        for (pattern, group) in groups(&run, |i| ending_pattern(&doc.sents[i])) {
            let n = group.len();
            if n < p.ending_run {
                continue;
            }
            let ranges = group
                .iter()
                .map(|&i| last_word(doc, &doc.sents[i]))
                .collect();
            out.push(Issue::new(
                "A2",
                Severity::Mid,
                doc.sents[group[0]].para,
                format!(
                    "‘-{pattern}.’{} 끝나는 문장이 {n}번 이어짐",
                    hangul::euro(&pattern)
                ),
                "한 문장은 현재형이나 명사형으로 끝내 보기",
                ranges,
            ));
        }
    }
}

const NOUN_STOP: &[&str] = &[
    "것", "수", "때", "곳", "앞", "뒤", "안", "위", "속", "쪽", "중", "번", "듯", "채", "줄", "말",
    "일",
];
const VERB_STOP: &[&str] = &[
    "있", "하", "되", "없", "같", "않", "보", "오", "가", "알", "주", "들", "나", "모르", "싶",
    "이", "아니",
];

/// A3: the same content word several times within a few words.
pub(super) fn echoes(doc: &Doc, names: &Names, p: &Profile, out: &mut Vec<Issue>) {
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
                by_lemma.entry(lemma).or_default().push(Occ {
                    word: word_index(t.start),
                    range: (t.start, t.end),
                    para: s.para,
                });
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
