//! Time anchoring (시각 고정, docs/creation-proof.md §4.2): once a day the
//! fingerprints of every device's journal and every chapter are folded into
//! one Merkle root, and public time-stamping authorities (RFC 3161) sign
//! that root with the time. Only those 32 bytes leave the device; nothing
//! of the manuscript can be read back from them.
//!
//! ```text
//! .journal/anchors/<stamp>-<device>.json        the leaves, the root, the tokens
//! .journal/anchors/<stamp>-<device>-<tsa>.tsr   each authority's reply (DER)
//! ```
//!
//! `<stamp>` is the UTC time the request was made (`store::stamp`). The
//! device is in the name so two devices anchoring on the same day never
//! write the same file. Each signed token also adds an `anchor` line to the
//! journal. The app does the sending (`prepare` → HTTP → `finish`), so this
//! module never touches the network and is tested offline.
//!
//! A journal leaf is the hash of the device's last line that is not itself
//! an `anchor` line: anchoring alone then changes nothing, and a day
//! without writing needs no new stamp.

mod merkle;
pub mod tsp;

pub use merkle::{Leaf, root as merkle_root};

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::journal::{self, JOURNAL_DIR, fingerprint, hex};
use crate::markup::write_body;
use crate::store::{atomic_write, parse_iso, sha256, stamp, to_iso};
use crate::{Error, Result, doc, project};

pub const ANCHOR_DIR: &str = "anchors";
const RECORD_FORMAT: u32 = 1;

/// A public time-stamping authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tsa {
    /// Short name used in file names and journal lines.
    pub name: &'static str,
    /// What the writer and the certificate call it.
    pub label: &'static str,
    pub url: &'static str,
}

/// The authorities asked, all free, taking plain `application/timestamp-query`
/// POSTs without an account. Three operators in three countries, so the
/// stamps still verify if one goes away (§4.2). Checked 2026-10-03; their
/// terms for automatic requests are still to be confirmed before release
/// (§9.3).
pub const TSAS: [Tsa; 3] = [
    Tsa {
        name: "digicert",
        label: "DigiCert",
        url: "http://timestamp.digicert.com",
    },
    Tsa {
        name: "sectigo",
        label: "Sectigo",
        url: "http://timestamp.sectigo.com",
    },
    Tsa {
        name: "freetsa",
        label: "FreeTSA",
        url: "https://freetsa.org/tsr",
    },
];

/// The display name of an authority by its short name.
pub fn tsa_label(name: &str) -> String {
    TSAS.iter()
        .find(|t| t.name == name)
        .map_or_else(|| name.to_string(), |t| t.label.to_string())
}

/// One anchoring: what was stamped and the tokens that came back.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub format: u32,
    /// When the request was made, by this device's clock.
    pub time: String,
    pub device: String,
    /// The Merkle root (hex) over `leaves`, in this order.
    pub root: String,
    pub leaves: Vec<Leaf>,
    pub tokens: Vec<TokenFile>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenFile {
    pub tsa: String,
    pub url: String,
    /// File name in `.journal/anchors/`.
    pub file: String,
    /// SHA-256 (hex) of the file.
    pub sha256: String,
    /// When the authority signed (UTC).
    pub gen_time: String,
}

fn dir(root: &Path) -> PathBuf {
    root.join(JOURNAL_DIR).join(ANCHOR_DIR)
}

/// The kind of a journal line, if it can be read.
fn kind_of(line: &[u8]) -> Option<String> {
    let v: Value = serde_json::from_slice(line).ok()?;
    v.get("kind")?.as_str().map(str::to_string)
}

/// The leaves for `root` as it is now, sorted by label: each device's last
/// line that is not an anchor line, and each manuscript chapter's body.
pub fn leaves(root: &Path) -> Result<Vec<Leaf>> {
    let mut out = Vec::new();
    for (device, lines) in journal::read_all(root)? {
        let last = lines
            .iter()
            .rev()
            .find(|line| kind_of(line).as_deref() != Some("anchor"));
        if let Some(line) = last {
            out.push(Leaf::journal(&device, &fingerprint(line)));
        }
    }
    let project = project::load(root)?;
    for id in project.parts.iter().flat_map(|p| &p.docs) {
        // A chapter still on its way from another device is left out.
        if let Ok(file) = doc::load(root, id) {
            out.push(Leaf::doc(
                id,
                &fingerprint(write_body(&file.body).as_bytes()),
            ));
        }
    }
    out.sort();
    out.dedup_by(|a, b| a.label == b.label);
    Ok(out)
}

/// Every anchoring record in `root`, oldest first. Files that cannot be read
/// are skipped (one cut short by a sync program, say).
pub fn records(root: &Path) -> Result<Vec<Record>> {
    let dir = dir(root);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(&dir, e)),
    };
    let mut out: Vec<Record> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter(|p| {
            !p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        })
        .filter_map(|p| fs::read(&p).ok())
        .filter_map(|bytes| serde_json::from_slice::<Record>(&bytes).ok())
        .collect();
    out.sort_by(|a, b| a.time.cmp(&b.time).then(a.device.cmp(&b.device)));
    Ok(out)
}

/// A token file's bytes. `file` must be a bare name from a record.
pub fn token_bytes(root: &Path, file: &str) -> Result<Vec<u8>> {
    let bare = !file.is_empty()
        && file
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.')
        && !file.contains("..");
    if !bare {
        return Err(Error::Invalid("올바르지 않은 시각 인증 파일".into()));
    }
    let path = dir(root).join(file);
    fs::read(&path).map_err(|e| Error::io(&path, e))
}

/// A request about to be sent.
#[derive(Debug, Clone)]
pub struct Pending {
    pub device: String,
    pub time: String,
    pub leaves: Vec<Leaf>,
    pub root: [u8; 32],
    pub nonce: u64,
}

impl Pending {
    /// The TimeStampReq (DER) to POST to every authority.
    pub fn request(&self) -> Vec<u8> {
        tsp::request(&self.root, self.nonce)
    }
}

/// Why nothing needs sending now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Skip {
    /// No journal yet: nothing to vouch for.
    NoJournal,
    /// Nothing changed since the last stamp.
    Unchanged,
    /// A regular check, and too little written since today's last stamp.
    NotYet,
    /// This device has stamped `MOST_A_DAY` times today.
    Enough,
}

/// What sets off a stamp (docs/creation-proof.md §4.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Occasion {
    /// The regular look while a project is open: the first change of a new
    /// day, or `WRITTEN_ENOUGH` characters since the last stamp.
    Check,
    /// A moment worth a stamp of its own: a chapter marked finished or
    /// published, a manuscript sent or exported, the project or app closing.
    Moment,
    /// The writer asked for one now: no daily limit.
    Now,
}

/// Characters written since the last stamp that make a regular check stamp.
pub const WRITTEN_ENOUGH: u64 = 3_000;

/// Stamps a device takes in one day, besides the ones the writer asks for:
/// the public authorities are not to be asked too often.
pub const MOST_A_DAY: usize = 4;

/// What to send for `root` now, or why nothing. Nothing is sent when
/// nothing changed since the last stamp, whatever the occasion.
pub fn prepare(
    root: &Path,
    device: &str,
    occasion: Occasion,
) -> Result<std::result::Result<Pending, Skip>> {
    // Saves still gathering go in first, so the journal leaf covers them.
    journal::flush(root);
    prepare_at(root, device, occasion, Utc::now())
}

/// Characters this device added in saves after `since` (the journal's save
/// lines: never the text, only how much it grew).
fn written_since(root: &Path, device: &str, since: &str) -> Result<u64> {
    let since = parse_iso(since);
    let journals = journal::read_all(root)?;
    let Some((_, lines)) = journals.iter().find(|(d, _)| d == device) else {
        return Ok(0);
    };
    Ok(lines
        .iter()
        .filter_map(|l| serde_json::from_slice::<Value>(l).ok())
        .filter(|v| v.get("kind").and_then(Value::as_str) == Some("save"))
        .filter(|v| {
            let time = v.get("time").and_then(Value::as_str).and_then(parse_iso);
            matches!((time, since), (Some(t), Some(s)) if t > s)
        })
        .filter_map(|v| v.get("added").and_then(Value::as_u64))
        .sum())
}

fn prepare_at(
    root: &Path,
    device: &str,
    occasion: Occasion,
    now: DateTime<Utc>,
) -> Result<std::result::Result<Pending, Skip>> {
    let leaves = leaves(root)?;
    if !leaves.iter().any(|l| l.device().is_some()) {
        return Ok(Err(Skip::NoJournal));
    }
    let merkle = merkle_root(&leaves);
    let past = records(root)?;
    if past.last().is_some_and(|r| r.root == hex(&merkle)) {
        return Ok(Err(Skip::Unchanged));
    }
    let today = now.with_timezone(&Local).date_naive();
    let local_day = |r: &Record| parse_iso(&r.time).map(|t| t.with_timezone(&Local).date_naive());
    let mine: Vec<&Record> = past.iter().filter(|r| r.device == device).collect();
    let today_count = mine.iter().filter(|r| local_day(r) == Some(today)).count();
    if occasion != Occasion::Now && today_count >= MOST_A_DAY {
        return Ok(Err(Skip::Enough));
    }
    if occasion == Occasion::Check
        && let Some(last) = mine.iter().max_by(|a, b| a.time.cmp(&b.time))
        && local_day(last) == Some(today)
        && written_since(root, device, &last.time)? < WRITTEN_ENOUGH
    {
        return Ok(Err(Skip::NotYet));
    }
    let time = to_iso(now);
    let seed = sha256(format!("{}{time}{}", hex(&merkle), crate::store::new_id()).as_bytes());
    let nonce = u64::from_be_bytes(seed[..8].try_into().expect("8 bytes"));
    Ok(Ok(Pending {
        device: device.into(),
        time,
        leaves,
        root: merkle,
        nonce,
    }))
}

/// What an anchoring came to.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    /// Authorities that signed (labels).
    pub signed: Vec<String>,
    /// Authorities that did not, with why.
    pub failed: Vec<(String, String)>,
    /// The record written, when at least one signed.
    pub record: Option<String>,
}

/// Checks each authority's reply to `pending` (sent by the app; `Err` when
/// it could not be reached), keeps the good ones with a record of the leaves,
/// and adds an `anchor` line per token to the device's journal.
pub fn finish(
    root: &Path,
    pending: &Pending,
    replies: Vec<(Tsa, std::result::Result<Vec<u8>, String>)>,
) -> Result<Outcome> {
    let base = format!(
        "{}-{}",
        stamp(parse_iso(&pending.time).unwrap_or_else(Utc::now)),
        pending.device
    );
    let root_hex = hex(&pending.root);
    let mut outcome = Outcome::default();
    let mut tokens = Vec::new();
    for (tsa, reply) in replies {
        let checked = reply.and_then(|bytes| {
            let stamp = tsp::check(&bytes)?;
            if !stamp.sha256 || stamp.imprint != pending.root {
                return Err("다른 지문에 서명함".into());
            }
            if stamp
                .nonce
                .as_deref()
                .is_some_and(|n| n != tsp::nonce_bytes(pending.nonce))
            {
                return Err("다른 요청의 응답".into());
            }
            Ok((bytes, stamp))
        });
        let (bytes, stamp) = match checked {
            Ok(ok) => ok,
            Err(why) => {
                outcome.failed.push((tsa.label.into(), why));
                continue;
            }
        };
        let file = format!("{base}-{}.tsr", tsa.name);
        atomic_write(&dir(root).join(&file), &bytes)?;
        tokens.push(TokenFile {
            tsa: tsa.name.into(),
            url: tsa.url.into(),
            file,
            sha256: fingerprint(&bytes),
            gen_time: to_iso(stamp.gen_time),
        });
        outcome.signed.push(tsa.label.into());
    }
    if tokens.is_empty() {
        return Ok(outcome);
    }
    let record = Record {
        format: RECORD_FORMAT,
        time: pending.time.clone(),
        device: pending.device.clone(),
        root: root_hex.clone(),
        leaves: pending.leaves.clone(),
        tokens: tokens.clone(),
    };
    let name = format!("{base}.json");
    let text = serde_json::to_string_pretty(&record).expect("records serialize");
    atomic_write(&dir(root).join(&name), text.as_bytes())?;
    for token in tokens {
        let entry = journal::Entry::Anchor(journal::Anchor {
            root: root_hex.clone(),
            tsa: token.tsa,
            file: token.file,
            token: token.sha256,
            gen_time: token.gen_time,
        });
        journal::append(root, &pending.device, &entry)?;
    }
    outcome.record = Some(name);
    Ok(outcome)
}

#[cfg(test)]
pub(crate) mod tests;
