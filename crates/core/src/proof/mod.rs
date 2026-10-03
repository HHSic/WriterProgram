//! Creation proof (창작 과정 증명서, docs/creation-proof.md §5–6): a page for
//! people and a file for machines, made only when the writer asks, with
//! only what the writer picks.
//!
//! - `facts`: what the journals say for the chosen chapters and period.
//! - `html`: the certificate page (self-contained, printable).
//! - `bundle`: `증명자료.json`, journal lines, stamps and fingerprints.
//! - `verify`: checks a proof file; the certificate runs it on its own file
//!   and `examples/verify_proof.rs` runs it for anyone else.
//!
//! The certificate states facts about how the work was written. It never
//! says anything about AI use (§2): every summary ends by saying so.

mod bundle;
mod excerpt;
mod facts;
mod html;
pub mod verify;

pub use bundle::{AnchorPart, Bundle, ChapterPrint, JournalPart, Line, TokenPart};
pub use facts::NOT_ABOUT_AI;
pub use verify::{Finding, Level, Verdict, check, verify};

use std::path::{Path, PathBuf};

use chrono::{DateTime, FixedOffset, Local, Offset, Utc};
use serde::{Deserialize, Serialize};

use crate::store::{atomic_write, safe_file_name, to_iso};
use crate::{Result, journal};

pub const HTML_NAME: &str = "창작 과정 증명서.html";
pub const BUNDLE_NAME: &str = "증명자료.json";

/// What the writer picked.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    /// Chapters covered; none for the whole work.
    #[serde(default)]
    pub docs: Option<Vec<String>>,
    /// First and last day covered (`2026-09-01`, the writer's local days),
    /// both included; none for no limit.
    #[serde(default)]
    pub from: Option<String>,
    #[serde(default)]
    pub to: Option<String>,
    /// Chapters whose draft and final text are compared on the page.
    #[serde(default)]
    pub excerpts: Vec<String>,
    /// Times on the page as dates only (privacy, §9.3). The proof file keeps
    /// exact times: they are part of what the chain and the stamps vouch for.
    #[serde(default)]
    pub dates_only: bool,
}

/// A certificate made in memory.
#[derive(Debug, Clone)]
pub struct Made {
    pub summary: String,
    pub html: String,
    /// `증명자료.json`.
    pub bundle: String,
    /// Its own check found nothing wrong.
    pub ok: bool,
}

fn local_offset() -> FixedOffset {
    Local::now().offset().fix()
}

/// The summary sentence alone, for the dialog to show before making.
pub fn preview(root: &Path, options: &Options) -> Result<String> {
    // Saves still gathering go in first, so the counts include them.
    journal::flush(root);
    let scope = facts::Scope::new(options, local_offset())?;
    let journals = journal::read_all(root)?;
    Ok(facts::summary(&facts::gather(root, &scope, &journals)?))
}

pub fn make(root: &Path, options: &Options) -> Result<Made> {
    journal::flush(root);
    make_at(root, options, local_offset(), Utc::now())
}

pub(crate) fn make_at(
    root: &Path,
    options: &Options,
    tz: FixedOffset,
    now: DateTime<Utc>,
) -> Result<Made> {
    let scope = facts::Scope::new(options, tz)?;
    let journals = journal::read_all(root)?;
    let facts = facts::gather(root, &scope, &journals)?;
    let summary = facts::summary(&facts);
    let bundle = bundle::build(root, &scope, &facts, &journals, &to_iso(now), options)?;
    let verdict = verify::check(&bundle, &[]);
    let excerpts = facts
        .chapters
        .iter()
        .filter(|c| options.excerpts.contains(&c.id))
        .map(|c| excerpt::excerpt(root, c))
        .collect::<Result<Vec<_>>>()?;
    let html = html::Page {
        facts: &facts,
        options,
        summary: &summary,
        verdict: &verdict,
        excerpts: &excerpts,
        made: now,
        tz,
        from: scope.from,
        to: scope.to,
    }
    .render();
    Ok(Made {
        summary,
        html,
        bundle: serde_json::to_string_pretty(&bundle).expect("bundles serialize"),
        ok: verdict.ok,
    })
}

/// Where a certificate was written.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Written {
    /// The folder made for it inside the one the writer picked.
    pub folder: String,
    pub html: String,
    pub bundle: String,
    pub summary: String,
    pub ok: bool,
}

/// Makes the certificate and writes it into a new folder inside `parent`:
/// `<title> 창작 과정 증명 <date>/` with the page and the proof file.
pub fn write(root: &Path, options: &Options, parent: &Path) -> Result<Written> {
    let made = make(root, options)?;
    let title = crate::project::load(root)?.title;
    let date = Local::now().format("%Y-%m-%d");
    let base = safe_file_name(&format!("{title} 창작 과정 증명 {date}"), "창작 과정 증명");
    let mut folder: PathBuf = parent.join(&base);
    let mut n = 2;
    while folder.exists() {
        folder = parent.join(format!("{base} ({n})"));
        n += 1;
    }
    let html = folder.join(HTML_NAME);
    let bundle = folder.join(BUNDLE_NAME);
    atomic_write(&html, made.html.as_bytes())?;
    atomic_write(&bundle, made.bundle.as_bytes())?;
    let text = |p: &Path| p.to_string_lossy().into_owned();
    Ok(Written {
        folder: text(&folder),
        html: text(&html),
        bundle: text(&bundle),
        summary: made.summary,
        ok: made.ok,
    })
}

#[cfg(test)]
mod tests;
