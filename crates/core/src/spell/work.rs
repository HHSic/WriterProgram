//! 맞춤법 검사 over the whole work (or some documents), listed like find
//! results. Locked chapters (완료 회차 잠금) are listed too and marked, so
//! the screen shows them but does not change them.

use std::path::Path;

use serde::Serialize;

use super::Speller;
use super::check::{Known, issue_ranges};
use crate::Result;
use crate::search::{MAX_MATCHES, Match, docs_in, find_in_blocks_by, readable};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocIssues {
    pub doc_id: String,
    pub locked: bool,
    pub issues: Vec<Match>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkCheck {
    pub docs: Vec<DocIssues>,
    pub total: usize,
    /// More were found than listed.
    pub truncated: bool,
    /// Documents that could not be read.
    pub skipped: usize,
}

/// Words the dictionary does not know in the documents `doc_ids` (all of
/// the project when none).
pub fn check_work(
    root: &Path,
    speller: &Speller,
    known: &Known,
    doc_ids: Option<Vec<String>>,
) -> Result<WorkCheck> {
    let mut docs = Vec::new();
    let mut total = 0;
    let mut truncated = false;
    let mut skipped = 0;
    for id in docs_in(root, doc_ids)? {
        let Some((_, file)) = readable(root, &id, &mut skipped) else {
            continue;
        };
        let left = MAX_MATCHES.saturating_sub(total);
        let issues = find_in_blocks_by(&file.body, left + 1, |text| {
            issue_ranges(speller, known, text)
        });
        if issues.len() > left {
            truncated = true;
        }
        let issues: Vec<Match> = issues.into_iter().take(left).collect();
        if !issues.is_empty() {
            total += issues.len();
            docs.push(DocIssues {
                doc_id: id,
                locked: file.meta.locked,
                issues,
            });
        }
        if truncated {
            break;
        }
    }
    Ok(WorkCheck {
        docs,
        total,
        truncated,
        skipped,
    })
}
