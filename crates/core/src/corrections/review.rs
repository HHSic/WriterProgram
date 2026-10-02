//! Reading a corrected file against what was sent.
//!
//! The file's text comes as cells (see `import::marked`). Three views of it
//! are compared with the text as sent:
//!
//! - the editor's own text without tracked insertions (`U`): what the file
//!   was before tracked changes, with every direct edit. Comparing it with
//!   the sent text tells which characters are the sent ones, so formatting
//!   the editor added shows: new strikethrough is "delete this", new
//!   underline, colour or highlight on unchanged text is "look here";
//! - the text the editor wants (`F`): tracked deletions and new
//!   strikethrough taken out, tracked insertions kept. Comparing it with
//!   the sent text gives the changes;
//! - the chapter as it is now, to tell which changes fall in paragraphs
//!   the writer changed after sending (겹침).

use std::collections::HashSet;
use std::fs;
use std::ops::Range;
use std::path::Path;

use chrono::Utc;

use super::compare::{self, classify, is_space, paragraphs};
use super::flat::{Flat, Mapper, block_cells};
use super::sent::{self, Received};
use super::{ChangeKind, ChapterReview, EditorNote, Found, Look, Review, Span, State};
use crate::import::marked::{self, Edit, Marked, PARA, SCENE};
use crate::markup::{Block, Mark};
use crate::store::{atomic_write, now_iso, rev_of, stamp};
use crate::{Error, Result, doc, journal};

const REVIEW_FILE: &str = "review.json";

/// The text as sent, without empty paragraphs (they do not come back from
/// a file), with what each character looked like.
struct Sent {
    flat: Flat,
    chars: Vec<char>,
    underline: Vec<bool>,
    strike: Vec<bool>,
    /// Block, start in `chars`, length (without the paragraph end).
    paras: Vec<(usize, usize, usize)>,
}

impl Sent {
    fn new(blocks: &[Block]) -> Sent {
        let mut s = Sent {
            flat: Flat::of(blocks),
            chars: Vec::new(),
            underline: Vec::new(),
            strike: Vec::new(),
            paras: Vec::new(),
        };
        for (i, block) in blocks.iter().enumerate() {
            let cells = block_cells(block);
            if cells.is_empty() {
                continue;
            }
            s.paras.push((i, s.chars.len(), cells.len()));
            for c in cells.iter() {
                s.chars.push(c.ch);
                s.underline.push(c.marks.contains(&Mark::Underline {}));
                s.strike.push(c.marks.contains(&Mark::Strike {}));
            }
            s.chars.push(PARA);
            s.underline.push(false);
            s.strike.push(false);
        }
        s
    }

    /// The chapter stream position of a position here, as a start.
    fn from(&self, p: usize) -> usize {
        if p >= self.chars.len() {
            return match self.paras.last() {
                Some((block, ..)) => self.flat.index(block + 1, 0),
                None => 0,
            };
        }
        let k = self.paras.partition_point(|(_, s, _)| *s <= p) - 1;
        let (block, start, _) = self.paras[k];
        self.flat.index(block, p - start)
    }

    /// The same as an end. Right after a paragraph end it stays before the
    /// empty paragraphs that follow (`keep_gap`) or takes them in.
    fn to(&self, p: usize, keep_gap: bool) -> usize {
        if keep_gap && p > 0 && p <= self.chars.len() && self.chars[p - 1] == PARA {
            let k = self.paras.partition_point(|(_, s, _)| *s < p) - 1;
            return self.flat.index(self.paras[k].0 + 1, 0);
        }
        self.from(p)
    }
}

fn is_insert(c: &marked::Cell) -> bool {
    matches!(c.tracked, Some(t) if t.edit == Edit::Insert)
}

fn is_delete(c: &marked::Cell) -> bool {
    matches!(c.tracked, Some(t) if t.edit == Edit::Delete)
}

fn words(s: &[char]) -> Vec<String> {
    s.split(|c| is_space(*c) || matches!(*c, PARA | SCENE | '\n'))
        .filter(|w| !w.is_empty())
        .map(|w| w.iter().collect())
        .collect()
}

/// Text for a list: paragraph ends as spaces.
fn show(s: &[char]) -> String {
    s.iter()
        .map(|c| if *c == PARA { ' ' } else { *c })
        .collect::<String>()
}

/// Gives out ids: c1, c2 … for changes, l… for looks, n… for notes.
#[derive(Default)]
struct Ids {
    changes: usize,
    looks: usize,
    notes: usize,
}

/// Compares the corrected file with what was sent, chapter by chapter.
/// `chapters` are (doc id, title); `now` the chapters as they are now
/// (none when deleted).
pub(crate) fn build(
    chapters: &[(String, String)],
    sent: &[Vec<Block>],
    now: &[Option<Vec<Block>>],
    file: &Marked,
) -> std::result::Result<Vec<ChapterReview>, String> {
    let sents: Vec<Sent> = sent.iter().map(|b| Sent::new(b)).collect();
    let cells = &file.cells;
    let mut u: Vec<char> = Vec::new();
    let mut u2c: Vec<usize> = Vec::new();
    for (c, cell) in cells.iter().enumerate() {
        if !is_insert(cell) {
            u.push(cell.ch);
            u2c.push(c);
        }
    }
    let up = paragraphs(&u);

    // Which chapters are in this file (one file may hold one chapter of many).
    let found: Vec<bool> = if sents.len() <= 1 {
        vec![true; sents.len()]
    } else {
        let have: HashSet<String> = words(&u).into_iter().collect();
        sents
            .iter()
            .map(|s| {
                let w = words(&s.chars);
                !w.is_empty() && w.iter().filter(|w| have.contains(*w)).count() * 2 >= w.len()
            })
            .collect()
    };
    let Some(first) = found.iter().position(|f| *f) else {
        return Err(
            "이 파일에서 보낸 원고를 찾지 못함 · 보낸 원고의 교정본인지 확인해 주세요".into(),
        );
    };

    // Pair the file's paragraphs with the sent ones to see where each
    // chapter's text is.
    let mut keys_a: Vec<&[char]> = Vec::new();
    let mut chap_of: Vec<usize> = Vec::new();
    for (i, s) in sents.iter().enumerate().filter(|(i, _)| found[*i]) {
        for r in paragraphs(&s.chars) {
            keys_a.push(&s.chars[r.start..r.end - 1]);
            chap_of.push(i);
        }
    }
    let keys_b: Vec<&[char]> = up
        .iter()
        .map(|r| u[r.clone()].strip_suffix(&[PARA]).unwrap_or(&u[r.clone()]))
        .collect();
    let mut owner = vec![first; up.len()];
    let mut last = first;
    for g in compare::align(&keys_a, &keys_b, &|i| chap_of[i] == chap_of[i + 1]) {
        if !g.a.is_empty() {
            last = chap_of[g.a.start];
        }
        let one_to_one = g.a.len() == g.b.len();
        for (n, j) in g.b.clone().enumerate() {
            owner[j] = if one_to_one {
                chap_of[g.a.start + n]
            } else {
                last
            };
        }
    }
    // Each chapter's cells run from the end of the paragraph before its
    // first one to the start of the next chapter's.
    let mut starts: Vec<Option<usize>> = vec![None; sents.len()];
    for (j, r) in up.iter().enumerate() {
        if starts[owner[j]].is_none() {
            starts[owner[j]] = Some(if j == 0 { 0 } else { u2c[r.start - 1] + 1 });
        }
    }
    if starts.iter().all(Option::is_none) {
        starts[first] = Some(0);
    }
    let ranges: Vec<Range<usize>> = (0..sents.len())
        .map(|i| match starts[i] {
            Some(s) => {
                let end = starts[i + 1..]
                    .iter()
                    .flatten()
                    .next()
                    .copied()
                    .unwrap_or(cells.len());
                s..end
            }
            None => 0..0,
        })
        .collect();
    // Notes go to the chapter their text is in.
    let note_chapter = |n: &marked::FileNote| -> usize {
        match n.on {
            Some((s, _)) => ranges
                .iter()
                .enumerate()
                .filter(|(i, _)| starts[*i].is_some())
                .rfind(|(_, r)| r.start <= s)
                .map_or(first, |(i, _)| i),
            None => first,
        }
    };

    let mut ids = Ids::default();
    let mut out = Vec::new();
    for (ch, s) in sents.iter().enumerate() {
        let (doc_id, title) = chapters[ch].clone();
        let now_flat = now[ch].as_ref().map(|b| Flat::of(b));
        let mut review = ChapterReview {
            doc_id,
            title,
            found: found[ch],
            gone: now_flat.is_none(),
            edited: now_flat
                .as_ref()
                .is_some_and(|f| f.chars() != s.flat.chars()),
            changes: Vec::new(),
            looks: Vec::new(),
            notes: Vec::new(),
        };
        if !found[ch] {
            out.push(review);
            continue;
        }
        let range = ranges[ch].clone();
        let cc = &cells[range.clone()];
        let mut uch: Vec<char> = Vec::new();
        let mut uc: Vec<usize> = Vec::new();
        let mut c2u: Vec<Option<usize>> = vec![None; cc.len()];
        for (i, cell) in cc.iter().enumerate() {
            if !is_insert(cell) {
                c2u[i] = Some(uch.len());
                uch.push(cell.ch);
                uc.push(i);
            }
        }
        if uch.last().is_some_and(|c| *c != PARA) {
            uch.push(PARA);
            uc.push(
                cc.iter()
                    .rposition(|c| c.ch == PARA)
                    .unwrap_or(cc.len() - 1),
            );
        }
        let mut s2u: Vec<Option<usize>> = vec![None; s.chars.len()];
        let mut u2s: Vec<Option<usize>> = vec![None; uch.len()];
        for op in compare::diff(&s.chars, &uch)
            .into_iter()
            .filter(|op| op.equal)
        {
            for (x, y) in op.a.zip(op.b) {
                s2u[x] = Some(y);
                u2s[y] = Some(x);
            }
        }
        // New strikethrough: delete this.
        let struck: Vec<bool> = (0..uch.len())
            .map(|y| {
                let cell = &cc[uc[y]];
                cell.ch != PARA && cell.look.strike && u2s[y].is_none_or(|x| !s.strike[x])
            })
            .collect();

        // Unchanged text the editor marked: look here.
        let mut marks: Vec<Option<(bool, bool, bool)>> = vec![None; s.chars.len()];
        for (y, x) in u2s.iter().enumerate() {
            let Some(x) = *x else { continue };
            let cell = &cc[uc[y]];
            if cell.ch == PARA || struck[y] || is_delete(cell) {
                continue;
            }
            let underline = cell.look.underline && !s.underline[x];
            if underline || cell.look.color || cell.look.shade {
                marks[x] = Some((underline, cell.look.color, cell.look.shade));
            }
        }

        // The text the editor wants.
        let mut fch: Vec<char> = Vec::new();
        let mut f2c: Vec<Option<usize>> = Vec::new();
        for (i, cell) in cc.iter().enumerate() {
            if is_delete(cell) || c2u[i].is_some_and(|y| struck[y]) {
                continue;
            }
            if cell.ch == PARA && fch.last().is_none_or(|c| *c == PARA) {
                continue;
            }
            fch.push(cell.ch);
            f2c.push(Some(i));
        }
        if fch.last().is_some_and(|c| *c != PARA) {
            fch.push(PARA);
            f2c.push(None);
        }

        // Changes: characters that differ, a word's worth at a time.
        let ops = compare::diff(&s.chars, &fch);
        let mut groups: Vec<(Range<usize>, Range<usize>)> = Vec::new();
        for (k, op) in ops.iter().enumerate() {
            if op.equal {
                continue;
            }
            if let Some(last) = groups.last_mut()
                && k > 0
                && ops[k - 1].equal
                && ops[k - 1].a.start == last.0.end
                && !s.chars[ops[k - 1].a.clone()]
                    .iter()
                    .any(|c| is_space(*c) || matches!(*c, PARA | SCENE | '\n'))
            {
                last.0.end = op.a.end;
                last.1.end = op.b.end;
                continue;
            }
            groups.push((op.a.clone(), op.b.clone()));
        }
        let mapper = now_flat
            .as_ref()
            .map(|f| (f, Mapper::new(&s.flat.chars(), &[], &f.chars())));
        let place = |r: Range<usize>| -> Option<Span> {
            let (f, m) = mapper.as_ref()?;
            m.map(r).map(|r| f.span(r))
        };
        for (ra, rb) in groups {
            let before: String = s.chars[ra.clone()].iter().collect();
            let after: String = fch[rb.clone()].iter().collect();
            let kind = if before.is_empty() {
                ChangeKind::Insert
            } else if after.is_empty() {
                ChangeKind::Delete
            } else if before.contains(PARA) && !after.contains(PARA) {
                ChangeKind::Merge
            } else if after.contains(PARA) && !before.contains(PARA) {
                ChangeKind::Split
            } else {
                ChangeKind::Replace
            };
            let (mut tracked, mut strike, mut color) = (None, false, false);
            for x in ra.clone() {
                if let Some(y) = s2u[x] {
                    let cell = &cc[uc[y]];
                    match cell.tracked {
                        Some(t) if t.edit == Edit::Delete => tracked = tracked.or(Some(t.by)),
                        _ => strike |= struck[y],
                    }
                }
            }
            for f in rb.clone() {
                if let Some(i) = f2c[f] {
                    let cell = &cc[i];
                    match cell.tracked {
                        Some(t) if t.edit == Edit::Insert => tracked = tracked.or(Some(t.by)),
                        _ => color |= cell.look.color || cell.look.shade || cell.look.underline,
                    }
                }
            }
            let how = match (tracked, strike, color) {
                (Some(_), ..) => Found::Tracked,
                (None, true, _) => Found::Strike,
                (None, false, true) => Found::Color,
                _ => Found::Compared,
            };
            let who = tracked.and_then(|i| file.who.get(i));
            let keep_gap = before.ends_with(PARA) && (after.is_empty() || after.ends_with(PARA));
            let from = s.from(ra.start);
            let to = if ra.is_empty() {
                from
            } else {
                s.to(ra.end, keep_gap)
            };
            let lead_start = s.chars[..ra.start]
                .iter()
                .rposition(|c| *c == PARA)
                .map_or(0, |p| p + 1)
                .max(ra.start.saturating_sub(12));
            let trail_end = s.chars[ra.end..]
                .iter()
                .position(|c| *c == PARA)
                .map_or(s.chars.len(), |p| p + ra.end)
                .min(ra.end + 12);
            let now_span = place(from..to);
            ids.changes += 1;
            review.changes.push(super::Change {
                id: format!("c{}", ids.changes),
                kind,
                class: classify(&before, &after),
                before,
                after,
                lead: s.chars[lead_start..ra.start].iter().collect(),
                trail: s.chars[ra.end..trail_end].iter().collect(),
                at: s.flat.span(from..to),
                now: now_span,
                overlap: now_span.is_none(),
                how,
                author: who.map(|w| w.author.clone()).filter(|a| !a.is_empty()),
                date: who.map(|w| w.date.clone()).filter(|d| !d.is_empty()),
                state: State::Pending,
            });
        }

        // Looks: runs of marked characters within a paragraph.
        let mut x = 0;
        while x < s.chars.len() {
            if marks[x].is_none() {
                x += 1;
                continue;
            }
            let start = x;
            let mut flags = (false, false, false);
            while x < s.chars.len()
                && s.chars[x] != PARA
                && (marks[x].is_some() || is_space(s.chars[x]))
            {
                if let Some((a, b, c)) = marks[x] {
                    flags = (flags.0 || a, flags.1 || b, flags.2 || c);
                }
                x += 1;
            }
            let mut end = x;
            while end > start && is_space(s.chars[end - 1]) {
                end -= 1;
            }
            let (from, to) = (s.from(start), s.from(end));
            ids.looks += 1;
            let at = s.flat.span(from..to);
            review.looks.push(Look {
                id: format!("l{}", ids.looks),
                at,
                now: place(from..to),
                quote: show(&s.chars[start..end]),
                underline: flags.0,
                color: flags.1,
                highlight: flags.2,
            });
        }

        // The editor's notes on this chapter's text.
        for n in file.notes.iter().filter(|n| note_chapter(n) == ch) {
            let (at, quote) = match n.on {
                None => (None, String::new()),
                Some((cs, ce)) => {
                    let local = cs.saturating_sub(range.start).min(cc.len())
                        ..ce.saturating_sub(range.start).min(cc.len());
                    let hits: Vec<usize> = local
                        .clone()
                        .filter_map(|i| c2u[i].and_then(|y| u2s[y]))
                        .filter(|x| s.chars[*x] != PARA)
                        .collect();
                    let r = match (hits.iter().min(), hits.iter().max()) {
                        (Some(&a), Some(&b)) => s.from(a)..s.from(b + 1),
                        _ => {
                            let p = (0..local.start)
                                .rev()
                                .find_map(|i| c2u[i].and_then(|y| u2s[y]))
                                .map_or(0, |x| x + 1);
                            s.from(p)..s.from(p)
                        }
                    };
                    let quote: Vec<char> = cc[local]
                        .iter()
                        .filter(|c| !is_delete(c))
                        .map(|c| c.ch)
                        .collect();
                    (Some(r), show(&quote).trim().chars().take(200).collect())
                }
            };
            ids.notes += 1;
            review.notes.push(EditorNote {
                id: format!("n{}", ids.notes),
                text: n.text.clone(),
                author: n.author.clone(),
                date: n.date.clone(),
                at: at.clone().map(|r| s.flat.span(r)),
                now: at.and_then(place),
                quote,
                state: State::Pending,
                memo: None,
            });
        }
        out.push(review);
    }
    Ok(out)
}

fn review_path(root: &Path, exchange_id: &str) -> Result<std::path::PathBuf> {
    Ok(sent::dir(root, exchange_id)?.join(REVIEW_FILE))
}

pub(crate) fn save_review(root: &Path, review: &Review) -> Result<()> {
    let path = review_path(root, &review.exchange)?;
    let json = serde_json::to_string_pretty(review).expect("serializable");
    atomic_write(&path, json.as_bytes())
}

/// The last corrected file read for an exchange, with what was decided.
pub fn load_review(root: &Path, exchange_id: &str) -> Result<Option<Review>> {
    let path = review_path(root, exchange_id)?;
    match fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| Error::format(&path, e.to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::io(&path, e)),
    }
}

/// Reads a corrected file for an exchange and compares it with what was
/// sent. A copy of the file is kept with the record, and the result
/// replaces the last review.
pub fn read_corrected(root: &Path, exchange_id: &str, file: &Path) -> Result<Review> {
    let mut ex = sent::load(root, exchange_id)?;
    let marked = marked::read(file, &ex.scene_break).map_err(Error::Invalid)?;
    let bodies = sent::bodies(root, &ex)?;
    let now: Vec<Option<Vec<Block>>> = ex
        .chapters
        .iter()
        .map(|c| doc::load(root, &c.doc_id).ok().map(|d| d.body))
        .collect();
    let titles: Vec<(String, String)> = ex
        .chapters
        .iter()
        .map(|c| (c.doc_id.clone(), c.title.clone()))
        .collect();
    let chapters = build(&titles, &bodies, &now, &marked).map_err(Error::Invalid)?;

    let bytes = fs::read(file).map_err(|e| Error::io(file, e))?;
    let ext = file
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let stored = format!("received/{}.{ext}", stamp(Utc::now()));
    atomic_write(&sent::dir(root, exchange_id)?.join(&stored), &bytes)?;
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let at = now_iso();
    let review = Review {
        exchange: ex.id.clone(),
        file: name.clone(),
        stored: stored.clone(),
        at: at.clone(),
        chapters,
    };
    save_review(root, &review)?;
    ex.received.push(Received {
        at,
        name: name.clone(),
        stored,
        fingerprint: rev_of(&bytes),
    });
    sent::save(root, &ex)?;
    journal::note(
        root,
        journal::Entry::Exchange(journal::Exchange {
            exchange: ex.id.clone(),
            step: journal::ExchangeStep::Received,
            files: vec![journal::ExchangeFile {
                file: name,
                file_hash: journal::fingerprint(&bytes),
            }],
            docs: Vec::new(),
        }),
    );
    Ok(review)
}

/// The last review of an exchange with every undecided change, look and
/// note placed in the chapters as they are now: the writer may have
/// changed them, or accepted corrections, since the file was read. A
/// change whose paragraph the writer changed is 겹침 (`now` none).
pub fn current_review(root: &Path, exchange_id: &str) -> Result<Option<Review>> {
    let Some(mut review) = load_review(root, exchange_id)? else {
        return Ok(None);
    };
    place_now(root, &mut review)?;
    Ok(Some(review))
}

pub(crate) fn place_now(root: &Path, review: &mut Review) -> Result<()> {
    let ex = sent::load(root, &review.exchange)?;
    let bodies = sent::bodies(root, &ex)?;
    for chapter in &mut review.chapters {
        let Some(index) = ex.chapters.iter().position(|c| c.doc_id == chapter.doc_id) else {
            continue;
        };
        let pending = |s: State| s == State::Pending;
        let Ok(current) = doc::load(root, &chapter.doc_id) else {
            chapter.gone = true;
            for c in chapter.changes.iter_mut().filter(|c| pending(c.state)) {
                c.now = None;
            }
            for l in &mut chapter.looks {
                l.now = None;
            }
            for n in chapter.notes.iter_mut().filter(|n| pending(n.state)) {
                n.now = None;
            }
            continue;
        };
        chapter.gone = false;
        let sent = Flat::of(&bodies[index]);
        let accepted: Vec<(Range<usize>, Vec<char>)> = chapter
            .changes
            .iter()
            .filter(|c| c.state == State::Accepted)
            .map(|c| (sent.range_of(c.at), c.after.chars().collect()))
            .collect();
        let now = Flat::of(&current.body);
        let mapper = Mapper::new(&sent.chars(), &accepted, &now.chars());
        let place = |at: Span| mapper.map(sent.range_of(at)).map(|r| now.span(r));
        for c in chapter.changes.iter_mut().filter(|c| pending(c.state)) {
            c.now = place(c.at);
            c.overlap = c.now.is_none();
        }
        for l in &mut chapter.looks {
            l.now = place(l.at);
        }
        for n in chapter.notes.iter_mut().filter(|n| pending(n.state)) {
            n.now = n.at.and_then(place);
        }
    }
    Ok(())
}
