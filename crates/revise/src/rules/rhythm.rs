//! B. Length and rhythm: long sentences (B1), sentences of the same length
//! (B2), and paragraphs too dense or too thin (B3).

use super::helpers::{narration_runs, word_at};
use super::{Doc, Genre, Issue, Profile, Sent, Severity};
use crate::text::Kind;

#[rustfmt::skip]
const CUT_EC: &[&str] = &[
    "는데", "은데", "ㄴ데", "ᆫ데", "던데", "고", "며", "으며", "면서", "으면서", "지만", "는데도", "어서", "아서", "여서", "으니", "니",
];

/// B1: long sentences, with the places where they could be cut.
pub(super) fn long_sentences(doc: &Doc, p: &Profile, out: &mut Vec<Issue>) {
    for s in doc.sents.iter().filter(|s| s.kind == Kind::Narration) {
        let points: Vec<(usize, usize)> = s
            .toks
            .iter()
            .filter(|t| {
                t.morphs
                    .last()
                    .is_some_and(|m| m.pos == "EC" && CUT_EC.contains(&m.form.as_str()))
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
        let mut issue = Issue::new(
            "B1",
            Severity::Mid,
            s.para,
            format!("긴 문장 · {}자", s.chars),
            hint,
            vec![(s.start, s.end)],
        );
        issue.points = points;
        out.push(issue);
    }
}

/// B2: several sentences in a row with nearly the same length.
pub(super) fn rhythm(doc: &Doc, p: &Profile, out: &mut Vec<Issue>) {
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
                flagged[w..w + p.rhythm_sentences]
                    .iter_mut()
                    .for_each(|f| *f = true);
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
pub(super) fn dense_paragraphs(doc: &Doc, p: &Profile, out: &mut Vec<Issue>) {
    match p.genre {
        Genre::WebNovel => {
            for para in doc.paras.iter().filter(|x| !x.excluded) {
                let lines = doc.text[para.start..para.end]
                    .chars()
                    .count()
                    .div_ceil(p.mobile_line_chars);
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
                        run.iter()
                            .map(|&i| (doc.paras[i].start, doc.paras[i].end))
                            .collect(),
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
