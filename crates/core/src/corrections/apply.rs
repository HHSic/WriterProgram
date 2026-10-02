//! Accepting and rejecting corrections.
//!
//! Changes are placed in the chapter as it is now (see `flat::Mapper`): a
//! change is applied only where every paragraph it touches is as expected
//! (the text sent, plus changes accepted before). A change in a paragraph
//! the writer changed after sending (겹침) is never applied: it stays
//! pending and is reported in [`Applied::skipped`]. Before anything is
//! written, the chapter is kept as a `before-corrections` record.

use std::collections::HashSet;
use std::ops::Range;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::flat::{Flat, Mapper};
use super::review::{load_review, place_now, save_review};
use super::{ChangeClass, Review, State, sent};
use crate::doc::{self, DocFile};
use crate::import::marked::PARA;
use crate::markup::{Mark, MemoAttrs, write_body};
use crate::notes::{self, Anchor, NewNote};
use crate::store::new_id;
use crate::{Error, Result, journal, snapshot};

/// What the writer decided. Ids are those of the review.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Decisions {
    pub accept: Vec<String>,
    pub reject: Vec<String>,
    /// Accept every pending change of these classes that can be applied.
    pub accept_classes: Vec<ChangeClass>,
    /// Editor notes to keep as 메모, and to leave out.
    pub keep_notes: Vec<String>,
    pub drop_notes: Vec<String>,
}

/// A change or note that was not applied, and why (screen words).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Skipped {
    pub id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Applied {
    /// Chapters whose text changed.
    pub docs: Vec<String>,
    pub accepted: Vec<String>,
    pub rejected: Vec<String>,
    pub skipped: Vec<Skipped>,
    /// 메모 made from the editor's notes.
    pub memos: Vec<String>,
    /// The review with the decisions recorded.
    pub review: Review,
}

const OVERLAP: &str = "보낸 뒤 작가가 고친 문단이라 직접 고쳐 주세요";
const GONE: &str = "회차가 지워져서 반영할 수 없음";

/// Applies decisions to the chapters of an exchange's last review.
pub fn apply(root: &Path, exchange_id: &str, decisions: &Decisions) -> Result<Applied> {
    let mut review = load_review(root, exchange_id)?
        .ok_or_else(|| Error::Invalid("먼저 교정본을 불러와 주세요".into()))?;
    let ex = sent::load(root, exchange_id)?;
    let bodies = sent::bodies(root, &ex)?;
    let accept: HashSet<&str> = decisions.accept.iter().map(String::as_str).collect();
    let reject: HashSet<&str> = decisions.reject.iter().map(String::as_str).collect();
    let keep: HashSet<&str> = decisions.keep_notes.iter().map(String::as_str).collect();
    let drop: HashSet<&str> = decisions.drop_notes.iter().map(String::as_str).collect();
    let mut out = Applied {
        docs: Vec::new(),
        accepted: Vec::new(),
        rejected: Vec::new(),
        skipped: Vec::new(),
        memos: Vec::new(),
        review: review.clone(),
    };
    // Chapters written, for the creation journal.
    let mut written = Vec::new();

    for chapter in &mut review.chapters {
        let Some(index) = ex.chapters.iter().position(|c| c.doc_id == chapter.doc_id) else {
            continue;
        };
        for c in chapter.changes.iter_mut() {
            if c.state == State::Pending && reject.contains(c.id.as_str()) {
                c.state = State::Rejected;
                out.rejected.push(c.id.clone());
            }
        }
        for n in chapter.notes.iter_mut() {
            if n.state == State::Pending && drop.contains(n.id.as_str()) {
                n.state = State::Rejected;
            }
        }
        let wanted: Vec<usize> = chapter
            .changes
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                c.state == State::Pending
                    && (accept.contains(c.id.as_str())
                        || decisions.accept_classes.contains(&c.class))
            })
            .map(|(i, _)| i)
            .collect();
        let notes: Vec<usize> = chapter
            .notes
            .iter()
            .enumerate()
            .filter(|(_, n)| n.state == State::Pending && keep.contains(n.id.as_str()))
            .map(|(i, _)| i)
            .collect();
        if wanted.is_empty() && notes.is_empty() {
            continue;
        }
        let Ok(current) = doc::load(root, &chapter.doc_id) else {
            for &i in &wanted {
                out.skipped.push(Skipped {
                    id: chapter.changes[i].id.clone(),
                    reason: GONE.into(),
                });
            }
            for &i in &notes {
                out.skipped.push(Skipped {
                    id: chapter.notes[i].id.clone(),
                    reason: GONE.into(),
                });
            }
            continue;
        };
        let sent = Flat::of(&bodies[index]);
        let sent_chars = sent.chars();
        let accepted: Vec<(Range<usize>, Vec<char>)> = chapter
            .changes
            .iter()
            .filter(|c| c.state == State::Accepted)
            .map(|c| (sent.range_of(c.at), c.after.chars().collect()))
            .collect();
        let mut now = Flat::of(&current.body);
        let mapper = Mapper::new(&sent_chars, &accepted, &now.chars());

        // Where each accepted change goes now.
        let mut splices: Vec<(Range<usize>, Vec<char>)> = Vec::new();
        for &i in &wanted {
            let c = &mut chapter.changes[i];
            match mapper.map(sent.range_of(c.at)) {
                Some(r) => {
                    splices.push((r, c.after.chars().collect()));
                    c.state = State::Accepted;
                    out.accepted.push(c.id.clone());
                }
                None => {
                    c.overlap = true;
                    out.skipped.push(Skipped {
                        id: c.id.clone(),
                        reason: OVERLAP.into(),
                    });
                }
            }
        }
        // Notes kept: their text marked where it can still be found.
        let mut new_notes: Vec<(usize, NewNote)> = Vec::new();
        for &i in &notes {
            let n = &chapter.notes[i];
            let id = new_id();
            let place =
                n.at.and_then(|at| mapper.map(sent.range_of(at)))
                    .filter(|r| !r.is_empty());
            let anchor = match place {
                Some(r) => {
                    for cell in &mut now.cells[r] {
                        if cell.ch != PARA {
                            cell.marks.push(Mark::Memo {
                                attrs: MemoAttrs { id: id.clone() },
                            });
                            cell.marks.sort();
                        }
                    }
                    Anchor::Text
                }
                None => Anchor::Doc,
            };
            let who = if n.author.trim().is_empty() {
                "편집자".to_string()
            } else {
                n.author.trim().to_string()
            };
            new_notes.push((
                i,
                NewNote {
                    id: (anchor == Anchor::Text).then(|| id.clone()),
                    anchor,
                    target: chapter.doc_id.clone(),
                    quote: n.quote.clone(),
                    text: format!("{who}: {}", n.text),
                },
            ));
        }
        splices.sort_by_key(|(r, _)| std::cmp::Reverse(r.start));
        for (r, text) in &splices {
            now.splice(r.clone(), text);
        }
        let body = now.blocks();
        if body != current.body {
            written.push(journal::ExchangeDoc {
                doc: chapter.doc_id.clone(),
                body: journal::fingerprint(write_body(&body).as_bytes()),
            });
            let name = format!("교정 반영 전 · {}", review.file);
            snapshot::create(root, &current, "before-corrections", &name)?;
            let (_, path) = doc::locate(root, &chapter.doc_id)?;
            doc::write_doc_file(
                &path,
                &DocFile {
                    meta: current.meta.clone(),
                    body,
                },
            )?;
            out.docs.push(chapter.doc_id.clone());
        }
        for (i, spec) in new_notes {
            let made = notes::create(root, &spec)?;
            let n = &mut chapter.notes[i];
            n.state = State::Accepted;
            n.memo = Some(made.id.clone());
            out.memos.push(made.id);
        }
    }
    // What is still to decide, placed in the chapters as they are now.
    place_now(root, &mut review)?;
    save_review(root, &review)?;
    if !written.is_empty() {
        journal::note(
            root,
            journal::Entry::Exchange(journal::Exchange {
                exchange: review.exchange.clone(),
                step: journal::ExchangeStep::Applied,
                files: Vec::new(),
                docs: written,
            }),
        );
    }
    out.review = review;
    Ok(out)
}
