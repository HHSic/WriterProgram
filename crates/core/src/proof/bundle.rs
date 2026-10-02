//! The proof file (증명자료.json): what a verifier needs, and nothing of the
//! manuscript's text.
//!
//! ```text
//! { "format": 1, "kind": "writerprogram-creation-proof", "made": …, "title": …,
//!   "scope": { "docs": [..] | null, "from": "2026-09-01" | null, "to": … },
//!   "chapters": [{ "doc", "number", "title", "body", "chars" }],
//!   "journals": [{ "device", "lines": ["{…journal line…}", { "hash": "…" }, …] }],
//!   "anchors":  [{ "record": {…anchors/<name>.json…},
//!                  "tokens": [{ "tsa", "file", "reply": "<base64 DER>" }] }] }
//! ```
//!
//! Each journal goes from its first line up to the end of the period (or
//! further, to the last line a included stamp covers), so its hash chain
//! can be followed. Lines outside the chosen chapters or period appear only
//! as their SHA-256 (`{"hash"}`): the chain still links through them, and a
//! stamp over a later line still covers them, but what they say stays out.
//! Stamps are the ones made up to the end of the period, plus the first
//! one after it (that is the one that covers the period's last day).

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde::{Deserialize, Serialize};

use super::facts::{Event, Facts, Scope};
use crate::Result;
use crate::anchor::{self, Record};
use crate::journal::fingerprint;
use crate::store::parse_iso;

pub const FORMAT: u32 = 1;
pub const KIND: &str = "writerprogram-creation-proof";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bundle {
    pub format: u32,
    pub kind: String,
    /// When the certificate was made (UTC).
    pub made: String,
    pub title: String,
    pub scope: ScopeNote,
    pub chapters: Vec<ChapterPrint>,
    pub journals: Vec<JournalPart>,
    pub anchors: Vec<AnchorPart>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeNote {
    pub docs: Option<Vec<String>>,
    pub from: Option<String>,
    pub to: Option<String>,
}

/// A chapter covered, as it was when the certificate was made.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterPrint {
    pub doc: String,
    pub number: usize,
    pub title: String,
    /// SHA-256 (hex) of the body, as in the journal (`save`) and the stamps.
    pub body: String,
    pub chars: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalPart {
    pub device: String,
    pub lines: Vec<Line>,
}

/// A journal line as written, or only its SHA-256.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Line {
    Shown(String),
    Hidden { hash: String },
}

impl Line {
    pub fn hash(&self) -> String {
        match self {
            Line::Shown(text) => fingerprint(text.as_bytes()),
            Line::Hidden { hash } => hash.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorPart {
    pub record: Record,
    pub tokens: Vec<TokenPart>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenPart {
    pub tsa: String,
    pub file: String,
    /// The authority's reply (TimeStampResp, DER) in base64.
    pub reply: String,
}

impl TokenPart {
    pub fn bytes(&self) -> Option<Vec<u8>> {
        STANDARD.decode(&self.reply).ok()
    }
}

/// The stamps to include (see the module notes).
fn chosen_records(records: Vec<Record>, scope: &Scope) -> Vec<Record> {
    let Some(to) = scope.to else {
        return records;
    };
    let mut out = Vec::new();
    for r in records {
        let after = parse_iso(&r.time).is_some_and(|t| scope.date(t) > to);
        out.push(r);
        if after {
            break;
        }
    }
    out
}

pub(crate) fn build(
    root: &std::path::Path,
    scope: &Scope,
    facts: &Facts,
    journals: &[(String, Vec<Vec<u8>>)],
    made: &str,
    options: &super::Options,
) -> Result<Bundle> {
    let records = chosen_records(anchor::records(root)?, scope);
    let mut parts = Vec::new();
    for (device, lines) in journals {
        let events: Vec<Option<Event>> = lines.iter().map(|l| Event::parse(l, scope)).collect();
        let mut cut = match scope.to {
            None => lines.len().checked_sub(1),
            Some(to) => events
                .iter()
                .rposition(|e| e.as_ref().is_some_and(|e| e.date <= to)),
        };
        // Stretch to the lines the chosen stamps cover.
        for leaf in records.iter().flat_map(|r| &r.leaves) {
            if leaf.device() == Some(device.as_str())
                && let Some(i) = lines.iter().position(|l| fingerprint(l) == leaf.value)
            {
                cut = Some(cut.map_or(i, |c| c.max(i)));
            }
        }
        let Some(cut) = cut else {
            continue;
        };
        let shown = lines[..=cut]
            .iter()
            .zip(&events)
            .map(|(line, e)| {
                let show = e.as_ref().is_some_and(|e| {
                    e.kind == "anchor" || (scope.in_period(e.date) && scope.covers(&e.v))
                });
                match std::str::from_utf8(line) {
                    Ok(text) if show => Line::Shown(text.to_string()),
                    _ => Line::Hidden {
                        hash: fingerprint(line),
                    },
                }
            })
            .collect();
        parts.push(JournalPart {
            device: device.clone(),
            lines: shown,
        });
    }

    let mut anchors = Vec::new();
    for record in records {
        let tokens = record
            .tokens
            .iter()
            .filter_map(|t| {
                let bytes = anchor::token_bytes(root, &t.file).ok()?;
                Some(TokenPart {
                    tsa: t.tsa.clone(),
                    file: t.file.clone(),
                    reply: STANDARD.encode(bytes),
                })
            })
            .collect();
        anchors.push(AnchorPart { record, tokens });
    }

    Ok(Bundle {
        format: FORMAT,
        kind: KIND.into(),
        made: made.into(),
        title: facts.title.clone(),
        scope: ScopeNote {
            docs: options.docs.clone(),
            from: scope.from.map(|d| d.to_string()),
            to: scope.to.map(|d| d.to_string()),
        },
        chapters: facts
            .chapters
            .iter()
            .map(|c| ChapterPrint {
                doc: c.id.clone(),
                number: c.number,
                title: c.title.clone(),
                body: c.body.clone(),
                chars: c.chars,
            })
            .collect(),
        journals: parts,
        anchors,
    })
}
