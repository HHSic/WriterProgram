//! E. Setting cards: names one jamo away from a card's name (E1).

use super::helpers::trim_punct;
use super::{Doc, Fix, Issue, Names, Severity};
use crate::hangul;

#[rustfmt::skip]
const PARTICLES: &[&str] = &[
    "", "는", "은", "이", "가", "을", "를", "의", "에게", "한테", "와", "과", "도", "만", "야", "아", "씨", "님", "께", "에게서",
    "이는", "이가", "이를", "이의", "이도", "이와", "이만", "이야", "이에게", "이한테",
];

/// Whitespace-delimited words with their byte offsets in `s`.
fn split_words(s: &str) -> Vec<(usize, &str)> {
    // Each word is a slice of `s`, so its offset is how far its start lies
    // from the start of `s`.
    s.split_whitespace()
        .map(|w| (w.as_ptr() as usize - s.as_ptr() as usize, w))
        .collect()
}

/// E1: a name one jamo away from a setting-card name, used rarely, while the card name is common.
pub(super) fn name_typos(doc: &Doc, names: &Names, out: &mut Vec<Issue>) {
    let targets: Vec<&str> = names
        .all()
        .filter(|n| !n.contains(char::is_whitespace) && n.chars().count() >= 2)
        .collect();
    if targets.is_empty() {
        return;
    }
    let count = |w: &str| {
        doc.sents
            .iter()
            .map(|s| doc.text[s.start..s.end].matches(w).count())
            .sum::<usize>()
    };
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
                issue.fix = Some(Fix {
                    start,
                    end,
                    replacement: name.to_string(),
                });
                out.push(issue);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn words_with_offsets() {
        assert_eq!(
            super::split_words(" 가나  다\t라\u{3000}마 "),
            [(1, "가나"), (9, "다"), (13, "라"), (19, "마")]
        );
        assert!(super::split_words(" \n ").is_empty());
    }
}
