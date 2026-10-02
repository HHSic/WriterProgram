//! D. Dialogue: long runs of dialogue-only paragraphs (D1).

use super::{Doc, Issue, Profile, Severity};
use crate::text::Kind;

/// D1: many dialogue-only paragraphs in a row.
pub(super) fn dialogue_runs(doc: &Doc, p: &Profile, out: &mut Vec<Issue>) {
    let mut run: Vec<usize> = Vec::new();
    let flush = |run: &mut Vec<usize>, out: &mut Vec<Issue>| {
        if run.len() >= p.dialogue_run {
            out.push(Issue::new(
                "D1",
                Severity::Mid,
                run[0],
                format!(
                    "대사만 {}줄 이어짐 · 누가 말하는지 헷갈릴 수 있음",
                    run.len()
                ),
                "중간에 행동 묘사나 ‘누가 말했다’를 넣어 보기",
                run.iter()
                    .map(|&i| (doc.paras[i].start, doc.paras[i].end))
                    .collect(),
            ));
        }
        run.clear();
    };
    for para in doc.paras.iter().filter(|x| !x.excluded) {
        let mut kinds = doc
            .sents
            .iter()
            .filter(|s| s.para == para.index)
            .map(|s| s.kind)
            .peekable();
        let has_any = kinds.peek().is_some();
        if has_any && kinds.all(|k| k != Kind::Narration) {
            run.push(para.index);
        } else {
            flush(&mut run, out);
        }
    }
    flush(&mut run, out);
}
