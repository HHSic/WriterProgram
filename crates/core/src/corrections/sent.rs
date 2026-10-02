//! What was sent: `.exchanges/<id>/exchange.json` (when, which files, which
//! chapters with a fingerprint each, the corrected files received) and
//! `sent/<doc id>.md`, each chapter's text exactly as it went out.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::EXCHANGE_DIR;
use super::review::load_review;
use crate::export::{self, DocOptions, ExportItem, FileKind};
use crate::format::ManuscriptFormat;
use crate::markup::{Block, parse_body, write_body};
use crate::store::{atomic_write, new_id, now_iso, read_text, rev_of};
use crate::{Error, Result, doc, journal};

const RECORD_FILE: &str = "exchange.json";
const SENT_DIR: &str = "sent";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SentChapter {
    pub doc_id: String,
    pub title: String,
    /// The heading line it was sent with.
    pub heading: String,
    /// The file it went in.
    pub file: String,
    /// Fingerprint of the text sent (see `store::rev_of`).
    pub fingerprint: String,
}

/// A corrected file read for this exchange.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Received {
    pub at: String,
    pub name: String,
    /// The copy kept with the record, relative to its folder.
    pub stored: String,
    pub fingerprint: String,
}

/// One sending of chapters to an editor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Exchange {
    pub id: String,
    pub created: String,
    pub kind: FileKind,
    /// Names of the files written.
    pub files: Vec<String>,
    /// Scene break symbol in the files.
    pub scene_break: String,
    pub chapters: Vec<SentChapter>,
    #[serde(default)]
    pub received: Vec<Received>,
}

/// An exchange in the list, with how many changes still wait for a
/// decision (none before a corrected file is read).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeInfo {
    #[serde(flatten)]
    pub exchange: Exchange,
    pub pending: Option<usize>,
}

pub(crate) fn dir(root: &Path, id: &str) -> Result<PathBuf> {
    let ok = !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric());
    if !ok {
        return Err(Error::Invalid("올바르지 않은 보낸 원고 기록".into()));
    }
    Ok(root.join(EXCHANGE_DIR).join(id))
}

pub(crate) fn save(root: &Path, ex: &Exchange) -> Result<()> {
    let json = serde_json::to_string_pretty(ex).expect("serializable");
    atomic_write(&dir(root, &ex.id)?.join(RECORD_FILE), json.as_bytes())
}

/// One exchange's record.
pub fn load(root: &Path, id: &str) -> Result<Exchange> {
    let path = dir(root, id)?.join(RECORD_FILE);
    if !path.exists() {
        return Err(Error::NotFound("보낸 원고 기록을 찾을 수 없음".into()));
    }
    serde_json::from_str(&read_text(&path)?).map_err(|e| Error::format(&path, e.to_string()))
}

/// The chapters' text as sent, in the record's order.
pub(crate) fn bodies(root: &Path, ex: &Exchange) -> Result<Vec<Vec<Block>>> {
    let sent = dir(root, &ex.id)?.join(SENT_DIR);
    ex.chapters
        .iter()
        .map(|c| {
            Ok(parse_body(&read_text(
                &sent.join(doc::file_name(&c.doc_id)),
            )?))
        })
        .collect()
}

/// Exports chapters for an editor ("편집자에게 보내기") like `export::export_file`,
/// and keeps what was sent.
pub fn send(
    root: &Path,
    items: &[ExportItem],
    opts: &DocOptions,
    format: &ManuscriptFormat,
    kind: FileKind,
    dest: &Path,
    per_doc: bool,
) -> Result<Exchange> {
    let docs = items
        .iter()
        .map(|item| doc::load(root, &item.doc_id))
        .collect::<Result<Vec<_>>>()?;
    let bodies: Vec<Vec<Block>> = docs.iter().map(|d| d.body.clone()).collect();
    let paths = export::export_bodies(root, items, &bodies, opts, format, kind, dest, per_doc)?;
    let name = |p: &PathBuf| {
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let base = root.join(EXCHANGE_DIR);
    let id = loop {
        let id = new_id();
        if !base.join(&id).exists() {
            break id;
        }
    };
    let sent = dir(root, &id)?.join(SENT_DIR);
    let mut chapters = Vec::new();
    let mut sent_docs = Vec::new();
    for (i, (item, file)) in items.iter().zip(&docs).enumerate() {
        let text = write_body(&file.body);
        atomic_write(&sent.join(doc::file_name(&item.doc_id)), text.as_bytes())?;
        sent_docs.push(journal::ExchangeDoc {
            doc: item.doc_id.clone(),
            body: journal::fingerprint(text.as_bytes()),
        });
        chapters.push(SentChapter {
            doc_id: item.doc_id.clone(),
            title: file.meta.title.clone(),
            heading: item.heading.clone(),
            file: name(&paths[if per_doc { i } else { 0 }]),
            fingerprint: rev_of(text.as_bytes()),
        });
    }
    let ex = Exchange {
        id,
        created: now_iso(),
        kind,
        files: paths.iter().map(name).collect(),
        scene_break: opts.scene_break.clone(),
        chapters,
        received: Vec::new(),
    };
    save(root, &ex)?;
    journal::note(
        root,
        journal::Entry::Exchange(journal::Exchange {
            exchange: ex.id.clone(),
            step: journal::ExchangeStep::Sent,
            files: paths
                .iter()
                .filter_map(|p| {
                    let bytes = fs::read(p).ok()?;
                    Some(journal::ExchangeFile {
                        file: name(p),
                        file_hash: journal::fingerprint(&bytes),
                    })
                })
                .collect(),
            docs: sent_docs,
        }),
    );
    Ok(ex)
}

/// Every exchange of a project, newest first.
pub fn list(root: &Path) -> Result<Vec<ExchangeInfo>> {
    let base = root.join(EXCHANGE_DIR);
    let entries = match fs::read_dir(&base) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(&base, e)),
    };
    let mut out = Vec::new();
    for entry in entries.filter_map(|e| e.ok()) {
        let id = entry.file_name().to_string_lossy().into_owned();
        // Folders that are not records (or half-synced ones) are passed over.
        let Ok(exchange) = load(root, &id) else {
            continue;
        };
        let pending = load_review(root, &id).ok().flatten().map(|r| r.pending());
        out.push(ExchangeInfo { exchange, pending });
    }
    out.sort_by(|a, b| b.exchange.created.cmp(&a.exchange.created));
    Ok(out)
}
