//! C. Expression: many '의' (C1), double passives (C2), translated-sounding
//! phrases (C3).

use super::helpers::{trim_punct, word_at};
use super::{Doc, Fix, Issue, Severity};
use crate::hangul;
use crate::text::Kind;

/// C1: many genitive particles '의' in one sentence.
pub(super) fn genitive(doc: &Doc, out: &mut Vec<Issue>) {
    for s in doc.sents.iter().filter(|s| s.kind == Kind::Narration) {
        let ranges: Vec<_> = s
            .toks
            .iter()
            .filter(|t| t.has_pos("JKG"))
            .map(|t| (t.start, t.end))
            .collect();
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
pub(super) fn double_passive(doc: &Doc, out: &mut Vec<Issue>) {
    for s in &doc.sents {
        let body = &doc.text[s.start..s.end];
        for &(stem, contracted) in DOUBLE_PASSIVE {
            let mut from = 0;
            while let Some(found) = body[from..].find(contracted) {
                let at = from + found;
                let after = at + contracted.len();
                from = after;
                let Some(next) = body[after..].chars().next() else {
                    continue;
                };
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
                let fixed = format!(
                    "{}{}{}",
                    &doc.text[word.0..start],
                    replacement,
                    &doc.text[end..word.1]
                );
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
                issue.fix = Some(Fix {
                    start,
                    end,
                    replacement,
                });
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
pub(super) fn translationese(doc: &Doc, out: &mut Vec<Issue>) {
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
                out.push(Issue::new(
                    "C3",
                    Severity::Low,
                    s.para,
                    format!("번역 투 표현 · ‘~{phrase}’"),
                    hint,
                    vec![r],
                ));
            }
        }
    }
}
