//! Draft-and-final excerpts (고친 흐름, §5.2 4): only for chapters the writer
//! chose to show. The draft is the chapter's oldest record, the middle one
//! the record halfway between, the final its text now. The paragraphs that
//! changed from draft to final are shown with what went and what came.

use std::path::Path;

use chrono::{DateTime, Utc};
use similar::{Algorithm, DiffTag, capture_diff_slices};

use super::facts::Chapter;
use crate::count::count_blocks;
use crate::markup::Block;
use crate::store::parse_iso;
use crate::{Result, snapshot};

/// Changed spots shown per chapter at most.
const MAX_SPOTS: usize = 12;

/// A stretch of text: kept, removed (draft only) or added (final only).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Piece {
    Same(String),
    Gone(String),
    New(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Stage {
    pub at: Option<DateTime<Utc>>,
    pub chars: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Excerpt {
    pub number: usize,
    pub title: String,
    pub draft: Option<Stage>,
    pub middle: Option<Stage>,
    pub final_chars: u32,
    /// Each changed spot: one or more paragraphs as pieces.
    pub spots: Vec<Vec<Piece>>,
    /// Changed spots left out past `MAX_SPOTS`.
    pub more: usize,
}

fn paragraphs(body: &[Block]) -> Vec<String> {
    body.iter()
        .map(|b| match b {
            Block::SceneBreak {} => "* * *".to_string(),
            _ => b.lines().join("\n"),
        })
        .collect()
}

/// Whole paragraphs, one after another.
fn with_breaks(pieces: impl Iterator<Item = Piece>) -> Vec<Piece> {
    let mut out = Vec::new();
    for piece in pieces {
        if !out.is_empty() {
            out.push(Piece::Same("\n".into()));
        }
        out.push(piece);
    }
    out
}

/// Character-level pieces turning `a` into `b`.
fn inline(a: &str, b: &str) -> Vec<Piece> {
    let ac: Vec<char> = a.chars().collect();
    let bc: Vec<char> = b.chars().collect();
    let mut out: Vec<Piece> = Vec::new();
    let text = |cs: &[char]| cs.iter().collect::<String>();
    for op in capture_diff_slices(Algorithm::Myers, &ac, &bc) {
        let (tag, ra, rb) = op.as_tag_tuple();
        match tag {
            DiffTag::Equal => out.push(Piece::Same(text(&ac[ra]))),
            DiffTag::Delete => out.push(Piece::Gone(text(&ac[ra]))),
            DiffTag::Insert => out.push(Piece::New(text(&bc[rb]))),
            DiffTag::Replace => {
                out.push(Piece::Gone(text(&ac[ra])));
                out.push(Piece::New(text(&bc[rb])));
            }
        }
    }
    out
}

pub(crate) fn excerpt(root: &Path, chapter: &Chapter) -> Result<Excerpt> {
    let records = snapshot::list(root, &chapter.id)?;
    let stage = |info: &snapshot::SnapshotInfo| Stage {
        at: parse_iso(&info.at),
        chars: info.counts.with_spaces,
    };
    let draft_info = records.last();
    let middle = (records.len() >= 3).then(|| stage(&records[records.len() / 2]));
    let mut ex = Excerpt {
        number: chapter.number,
        title: chapter.title.clone(),
        draft: draft_info.map(stage),
        middle,
        final_chars: count_blocks(&chapter.file.body).with_spaces,
        spots: Vec::new(),
        more: 0,
    };
    let Some(draft_info) = draft_info else {
        return Ok(ex);
    };
    let draft = snapshot::load(root, &chapter.id, &draft_info.id)?;
    let a = paragraphs(&draft.body);
    let b = paragraphs(&chapter.file.body);
    let mut spots = Vec::new();
    for op in capture_diff_slices(Algorithm::Patience, &a, &b) {
        let (tag, ra, rb) = op.as_tag_tuple();
        let spot: Vec<Piece> = match tag {
            DiffTag::Equal => continue,
            DiffTag::Delete => with_breaks(ra.map(|i| Piece::Gone(a[i].clone()))),
            DiffTag::Insert => with_breaks(rb.map(|j| Piece::New(b[j].clone()))),
            DiffTag::Replace => {
                let (la, lb) = (ra.len(), rb.len());
                let mut pieces = Vec::new();
                for k in 0..la.max(lb) {
                    if k > 0 {
                        pieces.push(Piece::Same("\n".into()));
                    }
                    match (ra.clone().nth(k), rb.clone().nth(k)) {
                        (Some(i), Some(j)) => pieces.extend(inline(&a[i], &b[j])),
                        (Some(i), None) => pieces.push(Piece::Gone(a[i].clone())),
                        (None, Some(j)) => pieces.push(Piece::New(b[j].clone())),
                        (None, None) => {}
                    }
                }
                pieces
            }
        };
        spots.push(spot);
    }
    ex.more = spots.len().saturating_sub(MAX_SPOTS);
    spots.truncate(MAX_SPOTS);
    ex.spots = spots;
    Ok(ex)
}
