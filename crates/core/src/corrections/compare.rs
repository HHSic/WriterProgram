//! Comparing the text that was sent with the text that came back.
//!
//! Both are streams in which every paragraph ends with [`PARA`]. Paragraphs
//! are paired first: identical ones by a patience diff, the rest by
//! similarity, allowing two paragraphs to have become one (merged) or one
//! two (split). Inside a pair the 어절 (runs between spaces) are compared,
//! and where they differ, the characters: Korean edits often change one
//! particle, and "그는" → "그가" should read as 는 → 가.
//!
//! The diff itself is the `similar` crate's (Myers, patience).

use std::ops::Range;

use similar::{Algorithm, DiffTag, capture_diff_slices};

use super::ChangeClass;
use crate::import::marked::{PARA, SCENE};

/// One stretch of the comparison: `a` (sent) and `b` (returned) are the
/// same text, or `a` became `b` (either may be empty).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Op {
    pub equal: bool,
    pub a: Range<usize>,
    pub b: Range<usize>,
}

/// Paragraphs paired with each other: indices into the paragraph lists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Group {
    pub a: Range<usize>,
    pub b: Range<usize>,
}

/// Spaces between words. A line break is not one: it is a token of its own.
pub(crate) fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\u{A0}' | '\u{2002}' | '\u{3000}')
}

/// Punctuation, quotes, brackets, ellipses and dashes, ASCII or not.
pub(crate) fn is_punct(c: char) -> bool {
    c.is_ascii_punctuation()
        || matches!(c,
            '\u{A1}' | '\u{AB}' | '\u{B7}' | '\u{BB}' | '\u{BF}'
            | '\u{2010}'..='\u{2027}'
            | '\u{2030}'..='\u{205E}'
            | '\u{3001}'..='\u{3003}'
            | '\u{3008}'..='\u{3011}'
            | '\u{3014}'..='\u{301F}'
            | '\u{30FB}'
            | '\u{FF01}'..='\u{FF0F}'
            | '\u{FF1A}'..='\u{FF20}'
            | '\u{FF3B}'..='\u{FF40}'
            | '\u{FF5B}'..='\u{FF65}')
}

/// 띄어쓰기 when only spaces differ, 문장부호 when only punctuation (and
/// spaces) differ, 문장 고침 otherwise.
pub(crate) fn classify(before: &str, after: &str) -> ChangeClass {
    let without =
        |s: &str, drop: &dyn Fn(char) -> bool| s.chars().filter(|c| !drop(*c)).collect::<String>();
    if without(before, &is_space) == without(after, &is_space) {
        return ChangeClass::Spacing;
    }
    let neither = |c: char| is_space(c) || is_punct(c);
    if without(before, &neither) == without(after, &neither) {
        ChangeClass::Punctuation
    } else {
        ChangeClass::Wording
    }
}

/// Paragraphs of a stream, each with its [`PARA`]. Text after the last
/// [`PARA`] is a paragraph without one.
pub(crate) fn paragraphs(s: &[char]) -> Vec<Range<usize>> {
    let mut out = Vec::new();
    let mut start = 0;
    for (i, c) in s.iter().enumerate() {
        if *c == PARA {
            out.push(start..i + 1);
            start = i + 1;
        }
    }
    if start < s.len() {
        out.push(start..s.len());
    }
    out
}

/// A paragraph's text without its [`PARA`].
fn body(s: &[char]) -> &[char] {
    s.strip_suffix(&[PARA]).unwrap_or(s)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Space,
    Word,
    Mark,
}

fn kind(c: char) -> Kind {
    if is_space(c) {
        Kind::Space
    } else if matches!(c, '\n' | PARA | SCENE) {
        Kind::Mark
    } else {
        Kind::Word
    }
}

/// Tokens: runs of spaces, 어절, and line breaks, paragraph ends and scene
/// breaks one by one.
fn tokens(s: &[char]) -> Vec<Range<usize>> {
    let mut out: Vec<Range<usize>> = Vec::new();
    for (i, c) in s.iter().enumerate() {
        let k = kind(*c);
        match out.last_mut() {
            Some(last) if k != Kind::Mark && kind(s[last.start]) == k => last.end = i + 1,
            _ => out.push(i..i + 1),
        }
    }
    out
}

/// Pairs of neighbouring characters in a paragraph, its two ends marked,
/// sorted: what paragraphs are compared by when pairing them.
fn bigrams(x: &[char]) -> Vec<u64> {
    let key = |a: char, b: char| (u64::from(u32::from(a)) << 32) | u64::from(u32::from(b));
    let mut out: Vec<u64> = Vec::with_capacity(x.len() + 1);
    let mut prev = '\u{1}';
    for &c in x.iter().chain(std::iter::once(&'\u{2}')) {
        out.push(key(prev, c));
        prev = c;
    }
    out.sort_unstable();
    out
}

/// The bigrams of two paragraphs joined by a space.
fn joined(x: &[char], y: &[char]) -> Vec<u64> {
    let mut glue = Vec::with_capacity(x.len() + y.len() + 1);
    glue.extend_from_slice(x);
    glue.push(' ');
    glue.extend_from_slice(y);
    bigrams(&glue)
}

/// How alike two paragraphs are, 0 to 1: the share of character pairs
/// they have in common (Dice). Cheap, so every pairing can be weighed.
fn similarity(x: &[u64], y: &[u64]) -> f64 {
    if x.is_empty() && y.is_empty() {
        return 1.0;
    }
    let (mut i, mut j, mut same) = (0, 0, 0usize);
    while i < x.len() && j < y.len() {
        match x[i].cmp(&y[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                same += 1;
                i += 1;
                j += 1;
            }
        }
    }
    2.0 * same as f64 / (x.len() + y.len()) as f64
}

/// Cost of leaving a paragraph unpaired. Two of them (one deleted, one
/// inserted) cost less than pairing paragraphs under 30% alike.
const UNPAIRED: f64 = 0.35;
/// Extra cost of a merge or split over a plain pair.
const REJOIN: f64 = 0.05;

/// Pairs paragraphs. `joinable(i)` says whether paragraphs `i` and `i + 1`
/// of `a` may have been merged into one (not across chapters).
pub(crate) fn align(a: &[&[char]], b: &[&[char]], joinable: &dyn Fn(usize) -> bool) -> Vec<Group> {
    let mut out = Vec::new();
    let mut stretch: Option<(Range<usize>, Range<usize>)> = None;
    let flush = |stretch: &mut Option<(Range<usize>, Range<usize>)>, out: &mut Vec<Group>| {
        if let Some((ra, rb)) = stretch.take() {
            out.extend(pair_up(a, b, ra, rb, joinable));
        }
    };
    for op in capture_diff_slices(Algorithm::Patience, a, b) {
        let (tag, ra, rb) = op.as_tag_tuple();
        if tag == DiffTag::Equal {
            flush(&mut stretch, &mut out);
            out.extend(ra.zip(rb).map(|(i, j)| Group {
                a: i..i + 1,
                b: j..j + 1,
            }));
        } else {
            match stretch.as_mut() {
                Some((sa, sb)) => {
                    sa.end = ra.end;
                    sb.end = rb.end;
                }
                None => stretch = Some((ra, rb)),
            }
        }
    }
    flush(&mut stretch, &mut out);
    out
}

/// Pairs the paragraphs of a stretch with no identical ones, by dynamic
/// programming within a band around the diagonal.
fn pair_up(
    a: &[&[char]],
    b: &[&[char]],
    ra: Range<usize>,
    rb: Range<usize>,
    joinable: &dyn Fn(usize) -> bool,
) -> Vec<Group> {
    let (n, m) = (ra.len(), rb.len());
    if n == 0 || m == 0 {
        return vec![Group { a: ra, b: rb }];
    }
    let band = n.abs_diff(m) as f64 + 8.0;
    let in_band = |i: usize, j: usize| (i as f64 * m as f64 / n as f64 - j as f64).abs() <= band;
    let at = |i: usize, j: usize| i * (m + 1) + j;
    let mut cost = vec![f64::INFINITY; (n + 1) * (m + 1)];
    let mut back = vec![(0usize, 0usize); (n + 1) * (m + 1)];
    cost[0] = 0.0;
    let ga: Vec<Vec<u64>> = a[ra.clone()].iter().map(|p| bigrams(p)).collect();
    let gb: Vec<Vec<u64>> = b[rb.clone()].iter().map(|p| bigrams(p)).collect();
    let pa = |i: usize| a[ra.start + i];
    let pb = |j: usize| b[rb.start + j];
    for i in 0..=n {
        for j in 0..=m {
            if (i, j) == (0, 0) || !in_band(i, j) {
                continue;
            }
            let mut best = (f64::INFINITY, (0, 0));
            let mut consider = |di: usize, dj: usize, step: f64| {
                let prev = cost[at(i - di, j - dj)];
                if prev + step < best.0 {
                    best = (prev + step, (di, dj));
                }
            };
            if i >= 1 {
                consider(1, 0, UNPAIRED);
            }
            if j >= 1 {
                consider(0, 1, UNPAIRED);
            }
            if i >= 1 && j >= 1 {
                consider(1, 1, 1.0 - similarity(&ga[i - 1], &gb[j - 1]));
            }
            if i >= 2 && j >= 1 && joinable(ra.start + i - 2) {
                let merged = joined(pa(i - 2), pa(i - 1));
                consider(2, 1, 1.0 - similarity(&merged, &gb[j - 1]) + REJOIN);
            }
            if i >= 1 && j >= 2 {
                let split = joined(pb(j - 2), pb(j - 1));
                consider(1, 2, 1.0 - similarity(&ga[i - 1], &split) + REJOIN);
            }
            cost[at(i, j)] = best.0;
            back[at(i, j)] = best.1;
        }
    }
    let mut groups = Vec::new();
    let (mut i, mut j) = (n, m);
    while i > 0 || j > 0 {
        let (di, dj) = back[at(i, j)];
        groups.push(Group {
            a: ra.start + i - di..ra.start + i,
            b: rb.start + j - dj..rb.start + j,
        });
        i -= di;
        j -= dj;
    }
    groups.reverse();
    // Unpaired paragraphs next to each other read as one stretch.
    let mut out: Vec<Group> = Vec::new();
    for g in groups {
        let lone = |g: &Group| g.a.is_empty() || g.b.is_empty();
        match out.last_mut() {
            Some(last) if lone(last) && lone(&g) => {
                last.a.end = g.a.end;
                last.b.end = g.b.end;
            }
            _ => out.push(g),
        }
    }
    out
}

/// Adds an op, joining it to the last one of the same kind.
fn push(out: &mut Vec<Op>, op: Op) {
    if op.a.is_empty() && op.b.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut()
        && last.equal == op.equal
        && last.a.end == op.a.start
        && last.b.end == op.b.start
    {
        last.a.end = op.a.end;
        last.b.end = op.b.end;
        return;
    }
    out.push(op);
}

/// Compares two streams: paragraphs, then 어절, then characters.
pub(crate) fn diff(a: &[char], b: &[char]) -> Vec<Op> {
    let (pa, pb) = (paragraphs(a), paragraphs(b));
    let ka: Vec<&[char]> = pa.iter().map(|r| body(&a[r.clone()])).collect();
    let kb: Vec<&[char]> = pb.iter().map(|r| body(&b[r.clone()])).collect();
    let mut out = Vec::new();
    let span = |ps: &[Range<usize>], r: &Range<usize>, len: usize| -> Range<usize> {
        if r.is_empty() {
            let at = ps.get(r.start).map_or(len, |p| p.start);
            at..at
        } else {
            ps[r.start].start..ps[r.end - 1].end
        }
    };
    for g in align(&ka, &kb, &|_| true) {
        let (ca, cb) = (span(&pa, &g.a, a.len()), span(&pb, &g.b, b.len()));
        if a[ca.clone()] == b[cb.clone()] {
            push(
                &mut out,
                Op {
                    equal: true,
                    a: ca,
                    b: cb,
                },
            );
        } else {
            for op in words(&a[ca.clone()], &b[cb.clone()]) {
                push(
                    &mut out,
                    Op {
                        equal: op.equal,
                        a: op.a.start + ca.start..op.a.end + ca.start,
                        b: op.b.start + cb.start..op.b.end + cb.start,
                    },
                );
            }
        }
    }
    out
}

/// 어절 first, then the characters of the stretches that differ.
fn words(a: &[char], b: &[char]) -> Vec<Op> {
    let (ta, tb) = (tokens(a), tokens(b));
    let ka: Vec<&[char]> = ta.iter().map(|r| &a[r.clone()]).collect();
    let kb: Vec<&[char]> = tb.iter().map(|r| &b[r.clone()]).collect();
    let chars = |ts: &[Range<usize>], r: Range<usize>, len: usize| -> Range<usize> {
        if r.is_empty() {
            let at = ts.get(r.start).map_or(len, |t| t.start);
            at..at
        } else {
            ts[r.start].start..ts[r.end - 1].end
        }
    };
    let mut out = Vec::new();
    let mut pending: Option<(Range<usize>, Range<usize>)> = None;
    // A space alone between two changed stretches: which space pairs with
    // which is a coin toss ("비가 올것" → "비는 올 것"), so the stretches
    // are compared as one.
    let mut held: Option<(Range<usize>, Range<usize>)> = None;
    let flush = |pending: &mut Option<(Range<usize>, Range<usize>)>, out: &mut Vec<Op>| {
        if let Some((ra, rb)) = pending.take() {
            refine(a, b, chars(&ta, ra, a.len()), chars(&tb, rb, b.len()), out);
        }
    };
    let equal = |ra: Range<usize>, rb: Range<usize>, out: &mut Vec<Op>| {
        let (ca, cb) = (chars(&ta, ra, a.len()), chars(&tb, rb, b.len()));
        push(
            out,
            Op {
                equal: true,
                a: ca,
                b: cb,
            },
        );
    };
    for op in capture_diff_slices(Algorithm::Myers, &ka, &kb) {
        let (tag, ra, rb) = op.as_tag_tuple();
        if tag == DiffTag::Equal {
            let spaces = ra.clone().all(|t| kind(a[ta[t].start]) == Kind::Space);
            if pending.is_some() && held.is_none() && spaces {
                held = Some((ra, rb));
                continue;
            }
            flush(&mut pending, &mut out);
            if let Some((ha, hb)) = held.take() {
                equal(ha, hb, &mut out);
            }
            equal(ra, rb, &mut out);
        } else {
            if let Some((ha, hb)) = held.take()
                && let Some((pa, pb)) = pending.as_mut()
            {
                pa.end = ha.end;
                pb.end = hb.end;
            }
            match pending.as_mut() {
                Some((pa, pb)) => {
                    pa.end = ra.end;
                    pb.end = rb.end;
                }
                None => pending = Some((ra, rb)),
            }
        }
    }
    flush(&mut pending, &mut out);
    if let Some((ha, hb)) = held.take() {
        equal(ha, hb, &mut out);
    }
    out
}

/// Characters of one changed stretch. When little is left in common the
/// stretch stays one change rather than scattered letters.
fn refine(a: &[char], b: &[char], ra: Range<usize>, rb: Range<usize>, out: &mut Vec<Op>) {
    let (sa, sb) = (&a[ra.clone()], &b[rb.clone()]);
    let ops = capture_diff_slices(Algorithm::Myers, sa, sb);
    let same: usize = ops
        .iter()
        .filter(|op| op.tag() == DiffTag::Equal)
        .map(|op| op.old_range().len())
        .sum();
    if sa.is_empty() || sb.is_empty() || 2 * same < sa.len().max(sb.len()) {
        push(
            out,
            Op {
                equal: false,
                a: ra,
                b: rb,
            },
        );
        return;
    }
    for op in ops {
        let (tag, oa, ob) = op.as_tag_tuple();
        push(
            out,
            Op {
                equal: tag == DiffTag::Equal,
                a: oa.start + ra.start..oa.end + ra.start,
                b: ob.start + rb.start..ob.end + rb.start,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stream(paras: &[&str]) -> Vec<char> {
        let mut v = Vec::new();
        for p in paras {
            v.extend(p.chars());
            v.push(PARA);
        }
        v
    }

    /// Changes as "before→after" with ¶ for paragraph ends.
    fn changes(a: &[&str], b: &[&str]) -> Vec<String> {
        let (sa, sb) = (stream(a), stream(b));
        let show = |s: &[char]| {
            s.iter()
                .map(|c| if *c == PARA { '¶' } else { *c })
                .collect::<String>()
        };
        diff(&sa, &sb)
            .into_iter()
            .filter(|op| !op.equal)
            .map(|op| format!("{}→{}", show(&sa[op.a]), show(&sb[op.b])))
            .collect()
    }

    #[test]
    fn particles_spacing_and_punctuation() {
        assert_eq!(
            changes(
                &["그는 웃으며 할수 있다고 말했다."],
                &["그가 웃으며 할 수 있다고 말했다!"]
            ),
            ["는→가", "→ ", ".→!"]
        );
        assert_eq!(classify("는", "가"), ChangeClass::Wording);
        assert_eq!(classify("", " "), ChangeClass::Spacing);
        assert_eq!(classify(".", "!"), ChangeClass::Punctuation);
        assert_eq!(classify("…… ", "…"), ChangeClass::Punctuation);
        assert_eq!(classify("¶", " "), ChangeClass::Wording);
    }

    #[test]
    fn rewritten_words_stay_whole() {
        assert_eq!(
            changes(&["오늘은 바다로 갔다."], &["오늘은 학교에 갔다."]),
            ["바다로→학교에"]
        );
    }

    #[test]
    fn merged_and_split_paragraphs() {
        assert_eq!(
            changes(
                &[
                    "첫 문단.",
                    "둘째 문단이 짧다.",
                    "셋째 문단도 있다.",
                    "넷째."
                ],
                &["첫 문단.", "둘째 문단이 짧다. 셋째 문단도 있다.", "넷째."]
            ),
            ["¶→ "]
        );
        assert_eq!(
            changes(
                &["하나. 둘이 이어진 긴 문단이다.", "끝."],
                &["하나.", "둘이 이어진 긴 문단이다.", "끝."]
            ),
            [" →¶"]
        );
    }

    #[test]
    fn whole_paragraphs_in_and_out() {
        assert_eq!(
            changes(
                &["가나다라.", "지울 문단입니다.", "마바사."],
                &["가나다라.", "마바사.", "새 문단을 넣었다."]
            ),
            ["지울 문단입니다.¶→", "→새 문단을 넣었다.¶"]
        );
        let groups = align(&[&['가'][..], &['나'][..]], &[&['가'][..]], &|_| true);
        assert_eq!(
            groups,
            [Group { a: 0..1, b: 0..1 }, Group { a: 1..2, b: 1..1 }]
        );
    }
}
