//! Creation journal (창작 일지): a quiet record of how a work was written, so
//! the writer can later show it (docs/creation-proof.md).
//!
//! Each device appends to its own journal, one JSON object per line, split
//! into one file per month (the UTC month of the line's time):
//!
//! ```text
//! .journal/<device id>.jsonl              before monthly pieces (kept as is)
//! .journal/<device id>/2026-10.jsonl      then one piece per month
//! .journal/<device id>/2026-11.jsonl
//! ```
//!
//! ```text
//! {"kind":"save","time":"2026-10-02T01:02:03.456Z","doc":"k7q2m9x4t1ab","body":"9f2c…","chars":5120,"added":42,"removed":0,"saves":12,"since":"2026-10-02T00:55:10.120Z","prev":"3b1a…"}
//! ```
//!
//! `prev` is the SHA-256 (hex) of the previous line's bytes, or 64 zeros on
//! the device's first line: a hash chain, so changing or removing a line in
//! the middle shows up in `verify`. The chain runs on across pieces: the
//! first line of a month links to the last line of the piece before it, so
//! the pieces read in order are one journal. Lines are only ever added; the
//! app never rewrites or removes them, and never writes to a piece once a
//! later one exists, so a sync program sends only the current month again.
//! Two devices never write the same file, so a sync program carrying the
//! folder between them has nothing to merge.
//!
//! Saves are gathered (`note`): the saves of one chapter within ten minutes
//! become one `save` line with how many there were (`saves`), when the first
//! was (`since`), the sums of `added` and `removed`, and the body and length
//! after the last. The line is written when the batch is ten minutes old,
//! when another chapter is saved, right before any other line for the
//! project, and when the project closes or the app quits (`flush`,
//! `flush_all`). Old lines without `saves` stand for one save.
//!
//! No manuscript text goes in: only ids, fingerprints, counts and times.
//!
//! The app keeps this device's id and the on/off switch in its settings
//! folder (`Settings`) and turns the journal on by telling this module the
//! device id (`set_device`); with none set, the hooks in `doc`, `snapshot`,
//! `import` and `corrections` write nothing.

use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, RwLock};
use std::thread;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::copies::is_id;
use crate::store::{atomic_write, new_id, now_iso, parse_iso, sha256, to_iso};
use crate::{Error, Result};

pub const JOURNAL_DIR: &str = ".journal";
const EXTENSION: &str = "jsonl";
/// The folder of time stamps inside the journal folder (`anchor` module):
/// never a device's folder.
const ANCHOR_DIR: &str = "anchors";
/// `prev` of a device's first line.
const NO_PREV: &str = "0000000000000000000000000000000000000000000000000000000000000000";
/// Pastes shorter than this are not worth a line (spec §3).
pub const PASTE_MIN_CHARS: u32 = 100;
/// Saves of one chapter within this many minutes become one line.
pub const BATCH_MINUTES: i64 = 10;

/// The device the journal is written for; none while the journal is off.
static DEVICE: RwLock<Option<String>> = RwLock::new(None);
/// The saves not yet written, one batch per project folder. Every line is
/// written while holding it, so two lines never chain to the same previous
/// one and a batch always goes in before the line that follows it.
static PENDING: Mutex<BTreeMap<PathBuf, Batch>> = Mutex::new(BTreeMap::new());

/// Saves of one chapter gathered into one line.
#[derive(Debug, Clone)]
struct Batch {
    device: String,
    doc: String,
    first: DateTime<Utc>,
    last: DateTime<Utc>,
    saves: u32,
    added: u32,
    removed: u32,
    /// Body fingerprint and length after the last save.
    body: String,
    chars: u32,
}

impl Batch {
    fn entry(&self) -> Entry {
        Entry::Save(Save {
            doc: self.doc.clone(),
            body: self.body.clone(),
            chars: self.chars,
            added: self.added,
            removed: self.removed,
            saves: Some(self.saves),
            since: Some(to_iso(self.first)),
        })
    }
}

fn pending() -> MutexGuard<'static, BTreeMap<PathBuf, Batch>> {
    PENDING.lock().unwrap_or_else(|p| p.into_inner())
}

/// Writes the batch waiting for `root`, if any. A batch that cannot be
/// written is dropped with a note in the log: it never stops the work.
fn flush_locked(pending: &mut BTreeMap<PathBuf, Batch>, root: &Path) {
    if let Some(batch) = pending.remove(root)
        && let Err(e) = write_line(root, &batch.device, &batch.entry(), &to_iso(batch.last))
    {
        eprintln!("창작 일지를 쓰지 못함: {}", e.user_message());
    }
}

/// Writes the saves waiting for `root` now: when the project closes, and
/// before reading the journal for a certificate or a time stamp.
pub fn flush(root: &Path) {
    flush_locked(&mut pending(), root);
}

/// Writes every waiting batch: when the app quits or the journal is switched.
pub fn flush_all() {
    let mut pending = pending();
    let roots: Vec<PathBuf> = pending.keys().cloned().collect();
    for root in roots {
        flush_locked(&mut pending, &root);
    }
}

/// Writes the batches that are `BATCH_MINUTES` old by `now`; the app calls
/// it every minute, so a batch never waits much longer than that.
pub fn flush_due(now: DateTime<Utc>) {
    let mut pending = pending();
    let due: Vec<PathBuf> = pending
        .iter()
        .filter(|(_, b)| now - b.first >= chrono::Duration::minutes(BATCH_MINUTES))
        .map(|(root, _)| root.clone())
        .collect();
    for root in due {
        flush_locked(&mut pending, &root);
    }
}

/// Turns the journal on for `device` (its id, see `new_device_id`), or off
/// with none. Called by the app at start and when the writer switches it.
/// Saves gathered so far are written first, for the device they were made on.
pub fn set_device(device: Option<String>) {
    flush_all();
    let mut slot = DEVICE.write().unwrap_or_else(|p| p.into_inner());
    *slot = device.filter(|d| is_id(d));
}

/// The device the journal is written for, if it is on.
pub fn device() -> Option<String> {
    DEVICE.read().unwrap_or_else(|p| p.into_inner()).clone()
}

/// This device's journal settings, kept by the app in its settings folder
/// (outside any project), so the device id stays the same for this device
/// and differs from every other one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// 12 random characters, like file ids; names this device's journal file.
    pub device: String,
    /// On unless the writer turned it off.
    #[serde(default = "on")]
    pub enabled: bool,
    /// The writer has been told the journal is kept (spec §7).
    #[serde(default)]
    pub noticed: bool,
    /// Whether a fingerprint may go to the time-stamping authorities
    /// (시각 고정, `anchor` module); none until the writer is asked.
    #[serde(default)]
    pub anchor: Option<bool>,
    /// Ask the writer each time a stamp is due instead of taking it.
    #[serde(default)]
    pub anchor_ask: bool,
    /// Say so (a small note in the corner) when a stamp came.
    #[serde(default = "on")]
    pub anchor_notify: bool,
}

fn on() -> bool {
    true
}

impl Settings {
    /// The device to write for: none while the journal is off.
    pub fn active_device(&self) -> Option<String> {
        self.enabled.then(|| self.device.clone())
    }
}

/// Reads the settings at `path`; the first time (no file, or no usable device
/// id in it) makes a new device id, on by default, and saves it.
pub fn load_settings(path: &Path) -> Result<Settings> {
    let found = match fs::read(path) {
        Ok(bytes) => serde_json::from_slice::<Settings>(&bytes).ok(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(e) => return Err(Error::io(path, e)),
    };
    if let Some(settings) = found.filter(|s| is_id(&s.device)) {
        return Ok(settings);
    }
    let settings = Settings {
        device: new_id(),
        enabled: true,
        noticed: false,
        anchor: None,
        anchor_ask: false,
        anchor_notify: true,
    };
    save_settings(path, &settings)?;
    Ok(settings)
}

pub fn save_settings(path: &Path, settings: &Settings) -> Result<()> {
    let text = serde_json::to_string_pretty(settings).expect("settings serialize");
    atomic_write(path, text.as_bytes())
}

/// One line of the journal, without its time and chain link.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    /// A document's text was saved.
    Save(Save),
    /// A writing session: from the first edit until five minutes without one.
    Session(Session),
    /// A paste of at least `PASTE_MIN_CHARS` characters.
    Paste(Paste),
    /// A file was imported into chapters.
    Import(Import),
    /// A record (기록) was kept.
    Snapshot(Snapshot),
    /// 교정 주고받기: chapters sent to an editor, a corrected file taken
    /// back, or corrections applied.
    Exchange(Exchange),
    /// A time-stamping authority signed the fingerprint of the journals and
    /// chapters (시각 고정, `anchor` module): one line per authority.
    Anchor(Anchor),
    // `verify` and `summary` read kinds they do not know, so older versions
    // keep checking newer journals.
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Save {
    pub doc: String,
    /// SHA-256 (hex) of the body as written (`markup::write_body`).
    pub body: String,
    /// Characters with spaces after the save.
    pub chars: u32,
    /// How much longer or shorter the text got with this save (with all the
    /// saves of a batch: the sums).
    pub added: u32,
    pub removed: u32,
    /// How many saves the line stands for; none on lines from before saves
    /// were gathered, which stand for one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub saves: Option<u32>,
    /// When the first of them was; the line's time is the last.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Session {
    pub doc: String,
    pub start: String,
    pub end: String,
    pub inserted: u32,
    pub deleted: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Paste {
    pub doc: String,
    pub chars: u32,
    /// The text came from outside the app (not a copy or cut made in it).
    pub outside: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Import {
    /// File name only, never the folder it was in.
    pub file: String,
    /// SHA-256 (hex) of the file's bytes.
    pub file_hash: String,
    /// Documents made from it.
    pub docs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub doc: String,
    /// The record's id (`<stamp>.<kind>`).
    pub snapshot: String,
    /// SHA-256 (hex) of the record's body.
    pub body: String,
    pub snapshot_kind: String,
}

/// Which step of 교정 주고받기 an `exchange` line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExchangeStep {
    /// Chapters went out to an editor (`files` written, `docs` as sent).
    Sent,
    /// A corrected file came back (`files`: that file).
    Received,
    /// Corrections were applied (`docs`: the chapters as they are after).
    Applied,
}

/// A file that went out or came back: its name only, never the folder.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeFile {
    pub file: String,
    /// SHA-256 (hex) of the file's bytes.
    pub file_hash: String,
}

/// A chapter and the SHA-256 (hex) of its text (`markup::write_body`).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ExchangeDoc {
    pub doc: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Exchange {
    /// The exchange's id (`.exchanges/<id>`).
    pub exchange: String,
    pub step: ExchangeStep,
    pub files: Vec<ExchangeFile>,
    pub docs: Vec<ExchangeDoc>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Anchor {
    /// The Merkle root sent (hex); the record beside the token has its leaves.
    pub root: String,
    /// Short name of the authority (`anchor::TSAS`).
    pub tsa: String,
    /// The token's file name in `.journal/anchors/`.
    pub file: String,
    /// SHA-256 (hex) of the token file.
    pub token: String,
    /// When the authority signed, as it says (UTC).
    pub gen_time: String,
}

impl Entry {
    pub fn kind(&self) -> &'static str {
        match self {
            Entry::Save(_) => "save",
            Entry::Session(_) => "session",
            Entry::Paste(_) => "paste",
            Entry::Import(_) => "import",
            Entry::Snapshot(_) => "snapshot",
            Entry::Exchange(_) => "exchange",
            Entry::Anchor(_) => "anchor",
        }
    }

    fn fields(&self) -> String {
        let json = match self {
            Entry::Save(e) => serde_json::to_string(e),
            Entry::Session(e) => serde_json::to_string(e),
            Entry::Paste(e) => serde_json::to_string(e),
            Entry::Import(e) => serde_json::to_string(e),
            Entry::Snapshot(e) => serde_json::to_string(e),
            Entry::Exchange(e) => serde_json::to_string(e),
            Entry::Anchor(e) => serde_json::to_string(e),
        }
        .expect("entries serialize");
        // `{"a":1,"b":2}` → `"a":1,"b":2`
        json[1..json.len() - 1].to_string()
    }
}

/// The line for `entry`: kind, time, its fields, then the link to `prev`.
fn encode(entry: &Entry, time: &str, prev: &str) -> String {
    let fields = entry.fields();
    let sep = if fields.is_empty() { "" } else { "," };
    format!(
        "{{\"kind\":\"{}\",\"time\":{}{sep}{fields},\"prev\":\"{prev}\"}}",
        entry.kind(),
        serde_json::to_string(time).expect("strings serialize"),
    )
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// SHA-256 of some content, as 64 hex digits.
pub fn fingerprint(bytes: &[u8]) -> String {
    hex(&sha256(bytes))
}

/// The single file a device wrote before monthly pieces.
fn single_file(root: &Path, device: &str) -> PathBuf {
    root.join(JOURNAL_DIR).join(format!("{device}.{EXTENSION}"))
}

/// The piece of a device's journal for `month` (`YYYY-MM`).
fn piece_file(root: &Path, device: &str, month: &str) -> PathBuf {
    root.join(JOURNAL_DIR)
        .join(device)
        .join(format!("{month}.{EXTENSION}"))
}

/// `YYYY-MM` from a piece's file name, if it is one.
fn month_of(name: &str) -> Option<&str> {
    let month = name.strip_suffix(&format!(".{EXTENSION}"))?;
    let b = month.as_bytes();
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    (b.len() == 7 && digits(0..4) && b[4] == b'-' && digits(5..7)).then_some(month)
}

/// The pieces of `device`'s journal, oldest first: the single file from
/// before monthly pieces, then the months in order. Other files in the
/// device's folder (a sync program's copy, say) are not part of it.
fn pieces(root: &Path, device: &str) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let single = single_file(root, device);
    if single.is_file() {
        out.push(single);
    }
    let dir = root.join(JOURNAL_DIR).join(device);
    if !dir.is_dir() {
        return Ok(out);
    }
    let entries = fs::read_dir(&dir).map_err(|e| Error::io(&dir, e))?;
    let mut months: Vec<(String, PathBuf)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            Some((month_of(&name)?.to_string(), e.path()))
        })
        .collect();
    months.sort();
    out.extend(months.into_iter().map(|(_, path)| path));
    Ok(out)
}

/// Adds `entry` to this device's journal in `root`, right away (after any
/// saves still waiting for that project).
pub fn append(root: &Path, device: &str, entry: &Entry) -> Result<()> {
    append_at(root, device, entry, &now_iso())
}

/// `append` with the line's time given (UTC, `store::to_iso`).
pub fn append_at(root: &Path, device: &str, entry: &Entry, time: &str) -> Result<()> {
    let mut pending = pending();
    flush_locked(&mut pending, root);
    write_line(root, device, entry, time)
}

/// Writes one line to the piece for its month; the caller holds `PENDING`.
/// A line whose month is before the newest piece's (the clock was put back)
/// goes into the newest piece: a past piece never changes.
fn write_line(root: &Path, device: &str, entry: &Entry, time: &str) -> Result<()> {
    if !is_id(device) || device == ANCHOR_DIR {
        return Err(Error::Invalid("올바르지 않은 기기 이름".into()));
    }
    let month = parse_iso(time)
        .unwrap_or_else(Utc::now)
        .format("%Y-%m")
        .to_string();
    let before = pieces(root, device)?;
    let newest = before
        .iter()
        .filter_map(|p| month_of(&p.file_name()?.to_string_lossy()).map(str::to_string))
        .max();
    let month = newest.filter(|n| *n > month).unwrap_or(month);
    let path = piece_file(root, device, &month);
    let dir = path.parent().expect("inside the device's folder");
    fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    let mut file = open_retry(&path).map_err(|e| Error::io(&path, e))?;
    let (last, torn) = last_line(&mut file).map_err(|e| Error::io(&path, e))?;
    let last = match last {
        Some(line) => Some(line),
        // A new month: the chain goes on from the last line of the piece
        // before (read only; that piece stays as it is, even cut short).
        None => last_of_pieces(before.iter().rev().filter(|p| **p != path))?,
    };
    let prev = last.map_or_else(|| NO_PREV.to_string(), |line| fingerprint(&line));
    let mut out = Vec::new();
    // A line cut short (the app stopped mid-write) is closed first: it stays,
    // and `verify` shows it, but the chain goes on from it.
    if torn {
        out.push(b'\n');
    }
    out.extend_from_slice(encode(entry, time, &prev).as_bytes());
    out.push(b'\n');
    file.write_all(&out).map_err(|e| Error::io(&path, e))?;
    file.sync_data().map_err(|e| Error::io(&path, e))
}

/// The last line of the first of `pieces` (newest first) that has one.
fn last_of_pieces<'a>(pieces: impl Iterator<Item = &'a PathBuf>) -> Result<Option<Vec<u8>>> {
    for path in pieces {
        let mut file = File::open(path).map_err(|e| Error::io(path, e))?;
        if let (Some(line), _) = last_line(&mut file).map_err(|e| Error::io(path, e))? {
            return Ok(Some(line));
        }
    }
    Ok(None)
}

/// Opens the journal for reading and appending, retrying briefly while a sync
/// program or virus scanner holds it (Windows).
fn open_retry(path: &Path) -> std::io::Result<File> {
    let mut tries = 0;
    loop {
        match OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(path)
        {
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied && tries < 4 => {
                tries += 1;
                thread::sleep(Duration::from_millis(40 * tries));
            }
            other => return other,
        }
    }
}

/// The last line of the file (without its line break), and whether it was
/// cut short (no line break after it). Reads only the end of the file.
fn last_line(file: &mut File) -> std::io::Result<(Option<Vec<u8>>, bool)> {
    let len = file.seek(SeekFrom::End(0))?;
    if len == 0 {
        return Ok((None, false));
    }
    let mut chunk = 4096u64;
    loop {
        let from = len.saturating_sub(chunk);
        file.seek(SeekFrom::Start(from))?;
        let mut tail = vec![0; (len - from) as usize];
        file.read_exact(&mut tail)?;
        let torn = tail.last() != Some(&b'\n');
        let body = if torn {
            &tail[..]
        } else {
            &tail[..tail.len() - 1]
        };
        match body.iter().rposition(|&b| b == b'\n') {
            Some(i) => return Ok((Some(body[i + 1..].to_vec()), torn)),
            None if from == 0 => return Ok((Some(body.to_vec()), torn)),
            None => chunk *= 4,
        }
    }
}

/// Adds `entry` to the journal when it is on; called from saves, records,
/// imports and 교정 주고받기. Saves are gathered into batches (module
/// docs); anything else is written at once, after the waiting batch. A
/// journal that cannot be written never stops the work itself.
pub(crate) fn note(root: &Path, entry: Entry) {
    if let Some(device) = device()
        && let Err(e) = note_at(root, &device, entry, Utc::now())
    {
        eprintln!("창작 일지를 쓰지 못함: {}", e.user_message());
    }
}

pub(crate) fn note_at(root: &Path, device: &str, entry: Entry, now: DateTime<Utc>) -> Result<()> {
    let mut pending = pending();
    let Entry::Save(save) = entry else {
        flush_locked(&mut pending, root);
        return write_line(root, device, &entry, &to_iso(now));
    };
    if !is_id(device) {
        return Err(Error::Invalid("올바르지 않은 기기 이름".into()));
    }
    if pending
        .get(root)
        .is_some_and(|b| b.doc != save.doc || b.device != device)
    {
        flush_locked(&mut pending, root);
    }
    let batch = pending.entry(root.to_path_buf()).or_insert_with(|| Batch {
        device: device.to_string(),
        doc: save.doc.clone(),
        first: now,
        last: now,
        saves: 0,
        added: 0,
        removed: 0,
        body: String::new(),
        chars: 0,
    });
    batch.last = batch.last.max(now);
    batch.saves += save.saves.unwrap_or(1);
    batch.added = batch.added.saturating_add(save.added);
    batch.removed = batch.removed.saturating_add(save.removed);
    batch.body = save.body;
    batch.chars = save.chars;
    if now - batch.first >= chrono::Duration::minutes(BATCH_MINUTES) {
        flush_locked(&mut pending, root);
    }
    Ok(())
}

/// A writing session or paste reported by the editor.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum EditorEvent {
    Session {
        doc: String,
        start: String,
        end: String,
        inserted: u32,
        deleted: u32,
    },
    Paste {
        doc: String,
        chars: u32,
        outside: bool,
    },
}

/// Turns an editor's report into an entry: checks it and puts the times in
/// the journal's form. None for what is not worth a line (a session with no
/// edits, a short paste).
pub fn editor_entry(event: EditorEvent) -> Result<Option<Entry>> {
    let check_doc = |doc: &str| {
        if is_id(doc) {
            Ok(())
        } else {
            Err(Error::Invalid("올바르지 않은 문서 이름".into()))
        }
    };
    Ok(match event {
        EditorEvent::Session {
            doc,
            start,
            end,
            inserted,
            deleted,
        } => {
            check_doc(&doc)?;
            let (Some(s), Some(e)) = (parse_iso(&start), parse_iso(&end)) else {
                return Err(Error::Invalid("올바르지 않은 시각".into()));
            };
            if e < s {
                return Err(Error::Invalid("끝이 시작보다 앞섬".into()));
            }
            (inserted > 0 || deleted > 0).then(|| {
                Entry::Session(Session {
                    doc,
                    start: to_iso(s),
                    end: to_iso(e),
                    inserted,
                    deleted,
                })
            })
        }
        EditorEvent::Paste {
            doc,
            chars,
            outside,
        } => {
            check_doc(&doc)?;
            (chars >= PASTE_MIN_CHARS).then_some(Entry::Paste(Paste {
                doc,
                chars,
                outside,
            }))
        }
    })
}

// ---------------------------------------------------------------------------
// Reading

/// The devices with a journal in `root`: a single file `<device>.jsonl`, a
/// folder `<device>/` of monthly pieces, or both. Sorted.
fn devices(root: &Path) -> Result<Vec<String>> {
    let dir = root.join(JOURNAL_DIR);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(&dir, e)),
    };
    let mut out: Vec<String> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let path = e.path();
            let device = if path.is_dir() {
                name
            } else if path.is_file() {
                name.strip_suffix(&format!(".{EXTENSION}"))?.to_string()
            } else {
                return None;
            };
            (is_id(&device) && device != ANCHOR_DIR).then_some(device)
        })
        .collect();
    out.sort();
    out.dedup();
    Ok(out)
}

/// A device's lines, without line breaks, over all its pieces in order.
/// Each piece is split on its own, so a piece's last line cut short stays
/// one line.
fn read_device(root: &Path, device: &str) -> Result<Vec<Vec<u8>>> {
    let mut out = Vec::new();
    for path in pieces(root, device)? {
        let bytes = fs::read(&path).map_err(|e| Error::io(&path, e))?;
        out.extend(lines(&bytes).into_iter().map(<[u8]>::to_vec));
    }
    Ok(out)
}

/// Every device's journal in `root`: (device id, its lines without line
/// breaks, all pieces in order), sorted by device.
pub fn read_all(root: &Path) -> Result<Vec<(String, Vec<Vec<u8>>)>> {
    devices(root)?
        .into_iter()
        .map(|device| {
            let lines = read_device(root, &device)?;
            Ok((device, lines))
        })
        .collect()
}

/// The `prev` of a device's first line.
pub const FIRST_PREV: &str = NO_PREV;

/// Lines of a journal file, without line breaks. A last line without a line
/// break (cut short) is included.
fn lines(bytes: &[u8]) -> Vec<&[u8]> {
    let body = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    if body.is_empty() {
        return Vec::new();
    }
    body.split(|&b| b == b'\n').collect()
}

/// What is wrong with a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Problem {
    /// Not a journal line (changed by hand, or cut short).
    Unreadable,
    /// Does not link to the line before it: that line was changed or removed.
    Broken,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileCheck {
    pub device: String,
    /// Lines over all the device's pieces.
    pub lines: usize,
    /// First line that is wrong (from 1), and what is wrong with it.
    pub first_bad: Option<(usize, Problem)>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub files: Vec<FileCheck>,
    /// Every file links up from its first line to its last.
    pub ok: bool,
}

fn check_file(device: &str, lines: &[Vec<u8>]) -> FileCheck {
    let mut prev = NO_PREV.to_string();
    let mut first_bad = None;
    for (i, line) in lines.iter().enumerate() {
        let link = serde_json::from_slice::<Value>(line).ok().and_then(|v| {
            let kind_ok = v.get("kind").is_some_and(Value::is_string);
            let time_ok = v
                .get("time")
                .and_then(Value::as_str)
                .is_some_and(|t| parse_iso(t).is_some());
            let link = v.get("prev").and_then(Value::as_str).map(str::to_string);
            link.filter(|_| kind_ok && time_ok)
        });
        let problem = match link {
            None => Some(Problem::Unreadable),
            Some(link) if link != prev => Some(Problem::Broken),
            Some(_) => None,
        };
        if let Some(problem) = problem {
            first_bad = Some((i + 1, problem));
            break;
        }
        prev = fingerprint(line);
    }
    FileCheck {
        device: device.into(),
        lines: lines.len(),
        first_bad,
    }
}

/// Checks the chain of every device's journal in `root`, across its pieces.
pub fn verify(root: &Path) -> Result<Report> {
    let checks: Vec<FileCheck> = read_all(root)?
        .iter()
        .map(|(device, lines)| check_file(device, lines))
        .collect();
    let ok = checks.iter().all(|c| c.first_bad.is_none());
    Ok(Report { files: checks, ok })
}

/// How many saves a `save` line stands for: its `saves`, or one on lines
/// from before saves were gathered.
pub fn saves_of(line: &Value) -> u64 {
    line.get("saves").and_then(Value::as_u64).unwrap_or(1)
}

/// What the journal holds, for the writer to see that it works.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    /// Time of the earliest line on any device.
    pub since: Option<String>,
    /// Saves on all devices (a gathered line counts its `saves`).
    pub saves: usize,
    pub sessions: usize,
    pub pastes: usize,
    pub imports: usize,
    /// 교정 주고받기 lines: chapters sent, files taken back, corrections applied.
    pub exchanges: usize,
    /// Time stamps (one line per authority) on all devices.
    pub anchors: usize,
    /// When the newest time stamp was signed, on any device.
    pub last_anchor: Option<String>,
    /// Devices with a journal in this project.
    pub devices: usize,
    /// Lines written by this device.
    pub this_device: usize,
}

pub fn summary(root: &Path, device: Option<&str>) -> Result<Summary> {
    let mut out = Summary::default();
    let mut since: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut last_anchor: Option<chrono::DateTime<chrono::Utc>> = None;
    for (name, lines) in read_all(root)? {
        out.devices += 1;
        if device == Some(name.as_str()) {
            out.this_device = lines.len();
        }
        for line in &lines {
            let Ok(v) = serde_json::from_slice::<Value>(line) else {
                continue;
            };
            match v.get("kind").and_then(Value::as_str) {
                Some("save") => out.saves += saves_of(&v) as usize,
                Some("session") => out.sessions += 1,
                Some("paste") => out.pastes += 1,
                Some("import") => out.imports += 1,
                Some("exchange") => out.exchanges += 1,
                Some("anchor") => {
                    out.anchors += 1;
                    let signed = v.get("genTime").and_then(Value::as_str);
                    if let Some(t) = signed.and_then(parse_iso)
                        && last_anchor.is_none_or(|l| t > l)
                    {
                        last_anchor = Some(t);
                    }
                }
                _ => {}
            }
            if let Some(t) = v.get("time").and_then(Value::as_str).and_then(parse_iso)
                && since.is_none_or(|s| t < s)
            {
                since = Some(t);
            }
        }
    }
    out.since = since.map(to_iso);
    out.last_anchor = last_anchor.map(to_iso);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The one piece a device has written so far.
    fn only_piece(root: &Path, device: &str) -> PathBuf {
        let pieces = pieces(root, device).unwrap();
        assert_eq!(pieces.len(), 1, "{pieces:?}");
        pieces.into_iter().next().unwrap()
    }

    fn save(doc: &str, chars: u32) -> Entry {
        Entry::Save(Save {
            doc: doc.into(),
            body: fingerprint(doc.as_bytes()),
            chars,
            added: chars,
            removed: 0,
            saves: None,
            since: None,
        })
    }

    #[test]
    fn lines_chain_to_the_one_before() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        append(root, "dev1aaaaaaaa", &save("d1", 10)).unwrap();
        append(root, "dev1aaaaaaaa", &save("d1", 20)).unwrap();
        append(
            root,
            "dev1aaaaaaaa",
            &Entry::Paste(Paste {
                doc: "d1".into(),
                chars: 150,
                outside: true,
            }),
        )
        .unwrap();
        let text = fs::read_to_string(only_piece(root, "dev1aaaaaaaa")).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("{\"kind\":\"save\",\"time\":\""));
        assert!(lines[0].ends_with(&format!(",\"prev\":\"{NO_PREV}\"}}")));
        assert!(lines[1].ends_with(&format!(
            ",\"prev\":\"{}\"}}",
            fingerprint(lines[0].as_bytes())
        )));
        assert!(lines[2].contains("\"kind\":\"paste\""));
        assert!(lines[2].contains("\"outside\":true"));
        let report = verify(root).unwrap();
        assert!(report.ok);
        assert_eq!(report.files[0].lines, 3);
        assert_eq!(report.files[0].device, "dev1aaaaaaaa");

        let sum = summary(root, Some("dev1aaaaaaaa")).unwrap();
        assert_eq!((sum.saves, sum.pastes, sum.devices), (2, 1, 1));
        assert_eq!(sum.this_device, 3);
        assert!(sum.since.is_some());
    }

    #[test]
    fn a_changed_middle_line_breaks_the_chain() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        for chars in [10, 20, 30, 40] {
            append(root, "dev1aaaaaaaa", &save("d1", chars)).unwrap();
        }
        let path = only_piece(root, "dev1aaaaaaaa");
        let text = fs::read_to_string(&path).unwrap();

        // One number changed in line 2: line 3 no longer links to it.
        let edited = text.replacen("\"chars\":20", "\"chars\":25", 1);
        fs::write(&path, &edited).unwrap();
        let report = verify(root).unwrap();
        assert!(!report.ok);
        assert_eq!(report.files[0].first_bad, Some((3, Problem::Broken)));

        // Line 2 removed: line 3 (now 2) links to a line that is not there.
        let removed: Vec<&str> = text
            .lines()
            .enumerate()
            .filter(|(i, _)| *i != 1)
            .map(|(_, l)| l)
            .collect();
        fs::write(&path, removed.join("\n") + "\n").unwrap();
        assert_eq!(
            verify(root).unwrap().files[0].first_bad,
            Some((2, Problem::Broken))
        );

        // Not a journal line at all.
        fs::write(&path, text.replacen("{\"kind\"", "{kind", 1)).unwrap();
        assert_eq!(
            verify(root).unwrap().files[0].first_bad,
            Some((1, Problem::Unreadable))
        );
    }

    #[test]
    fn a_line_cut_short_is_closed_and_the_chain_goes_on() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        append(root, "dev1aaaaaaaa", &save("d1", 10)).unwrap();
        let path = only_piece(root, "dev1aaaaaaaa");
        let mut file = OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(b"{\"kind\":\"sa").unwrap();
        drop(file);
        append(root, "dev1aaaaaaaa", &save("d1", 20)).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert_eq!(text.lines().count(), 3);
        let last = text.lines().nth(2).unwrap();
        assert!(last.ends_with(&format!(
            ",\"prev\":\"{}\"}}",
            fingerprint(b"{\"kind\":\"sa")
        )));
        assert_eq!(
            verify(root).unwrap().files[0].first_bad,
            Some((2, Problem::Unreadable))
        );
    }

    #[test]
    fn devices_write_their_own_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        append(root, "dev1aaaaaaaa", &save("d1", 10)).unwrap();
        append(root, "dev2bbbbbbbb", &save("d1", 12)).unwrap();
        append(root, "dev2bbbbbbbb", &save("d1", 14)).unwrap();
        let report = verify(root).unwrap();
        assert!(report.ok);
        assert_eq!(report.files.len(), 2);
        let sum = summary(root, Some("dev2bbbbbbbb")).unwrap();
        assert_eq!((sum.saves, sum.devices, sum.this_device), (3, 2, 2));
        assert!(append(root, "../x", &save("d1", 1)).is_err());
    }

    #[test]
    fn long_lines_are_found_at_the_end() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let docs: Vec<String> = (0..2000).map(|i| format!("doc{i:09}")).collect();
        let big = Entry::Import(Import {
            file: "초고.hwpx".into(),
            file_hash: fingerprint(b"x"),
            docs,
        });
        append(root, "dev1aaaaaaaa", &big).unwrap();
        append(root, "dev1aaaaaaaa", &save("d1", 1)).unwrap();
        assert!(verify(root).unwrap().ok);
    }

    #[test]
    fn exchange_lines_name_files_and_fingerprints_only() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let sent = Entry::Exchange(Exchange {
            exchange: "x1".into(),
            step: ExchangeStep::Sent,
            files: vec![ExchangeFile {
                file: "보낸 원고.hwpx".into(),
                file_hash: fingerprint(b"file"),
            }],
            docs: vec![ExchangeDoc {
                doc: "d1".into(),
                body: fingerprint(b"text"),
            }],
        });
        append_at(root, "dev1aaaaaaaa", &sent, "2026-10-03T01:00:00.000Z").unwrap();
        let received = Entry::Exchange(Exchange {
            exchange: "x1".into(),
            step: ExchangeStep::Received,
            files: vec![ExchangeFile {
                file: "교정본.hwpx".into(),
                file_hash: fingerprint(b"back"),
            }],
            docs: Vec::new(),
        });
        append(root, "dev1aaaaaaaa", &received).unwrap();
        let text = fs::read_to_string(only_piece(root, "dev1aaaaaaaa")).unwrap();
        let first = text.lines().next().unwrap();
        assert_eq!(
            first,
            format!(
                "{{\"kind\":\"exchange\",\"time\":\"2026-10-03T01:00:00.000Z\",\"exchange\":\"x1\",\"step\":\"sent\",\"files\":[{{\"file\":\"보낸 원고.hwpx\",\"fileHash\":\"{}\"}}],\"docs\":[{{\"doc\":\"d1\",\"body\":\"{}\"}}],\"prev\":\"{NO_PREV}\"}}",
                fingerprint(b"file"),
                fingerprint(b"text"),
            )
        );
        assert!(
            text.lines()
                .nth(1)
                .unwrap()
                .contains("\"step\":\"received\"")
        );
        assert!(verify(root).unwrap().ok);
        assert_eq!(summary(root, None).unwrap().exchanges, 2);
    }

    #[test]
    fn editor_events_are_checked() {
        let session = |start: &str, end: &str, inserted| EditorEvent::Session {
            doc: "d1".into(),
            start: start.into(),
            end: end.into(),
            inserted,
            deleted: 0,
        };
        let entry = editor_entry(session(
            "2026-10-02T01:00:00Z",
            "2026-10-02T01:20:00.5Z",
            300,
        ))
        .unwrap();
        assert_eq!(
            entry,
            Some(Entry::Session(Session {
                doc: "d1".into(),
                start: "2026-10-02T01:00:00.000Z".into(),
                end: "2026-10-02T01:20:00.500Z".into(),
                inserted: 300,
                deleted: 0,
            }))
        );
        assert_eq!(
            editor_entry(session("2026-10-02T01:00:00Z", "2026-10-02T01:00:00Z", 0)).unwrap(),
            None
        );
        assert!(editor_entry(session("2026-10-02T02:00:00Z", "2026-10-02T01:00:00Z", 1)).is_err());
        assert!(editor_entry(session("어제", "2026-10-02T01:00:00Z", 1)).is_err());
        let paste = |chars| EditorEvent::Paste {
            doc: "d1".into(),
            chars,
            outside: false,
        };
        assert_eq!(editor_entry(paste(99)).unwrap(), None);
        assert!(editor_entry(paste(100)).unwrap().is_some());
    }

    #[test]
    fn settings_keep_the_device_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("journal.json");
        let first = load_settings(&path).unwrap();
        assert!(first.enabled && !first.noticed);
        assert_eq!(first.device.len(), 12);
        assert_eq!(first.active_device(), Some(first.device.clone()));
        let off = Settings {
            enabled: false,
            ..first.clone()
        };
        save_settings(&path, &off).unwrap();
        let again = load_settings(&path).unwrap();
        assert_eq!(again, off);
        assert_eq!(again.active_device(), None);
        // A damaged file gets a new id rather than writing to a bad name.
        fs::write(&path, r#"{"device":"a/b"}"#).unwrap();
        assert_ne!(load_settings(&path).unwrap().device, "a/b");
    }

    #[test]
    fn times_are_utc() {
        let dir = tempfile::tempdir().unwrap();
        append_at(
            dir.path(),
            "dev1aaaaaaaa",
            &save("d1", 1),
            "2026-10-02T01:02:03.456Z",
        )
        .unwrap();
        let text = fs::read_to_string(only_piece(dir.path(), "dev1aaaaaaaa")).unwrap();
        assert!(text.starts_with(
            "{\"kind\":\"save\",\"time\":\"2026-10-02T01:02:03.456Z\",\"doc\":\"d1\","
        ));
    }

    const DEV: &str = "dev1aaaaaaaa";

    fn at(s: &str) -> DateTime<Utc> {
        parse_iso(s).unwrap()
    }

    fn values(root: &Path, device: &str) -> Vec<Value> {
        read_device(root, device)
            .unwrap()
            .iter()
            .map(|l| serde_json::from_slice(l).unwrap())
            .collect()
    }

    fn kinds_of(values: &[Value]) -> Vec<&str> {
        values.iter().map(|v| v["kind"].as_str().unwrap()).collect()
    }

    /// A save as `doc::save_body` reports it: one save, `chars` long after.
    fn saved(doc: &str, chars: u32, added: u32, removed: u32) -> Entry {
        Entry::Save(Save {
            doc: doc.into(),
            body: fingerprint(format!("{doc}{chars}").as_bytes()),
            chars,
            added,
            removed,
            saves: None,
            since: None,
        })
    }

    #[test]
    fn a_hundred_saves_become_a_few_lines() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let start = at("2026-10-02T01:00:00.000Z");
        // One save every 15 seconds for 25 minutes: 5 longer, every third 2 shorter.
        let mut chars = 0u32;
        let (mut added, mut removed) = (0u64, 0u64);
        for i in 0..100u32 {
            let (a, r) = if i % 3 == 2 { (0, 2) } else { (5, 0) };
            chars = chars + a - r;
            added += u64::from(a);
            removed += u64::from(r);
            let now = start + chrono::Duration::seconds(15 * i64::from(i));
            note_at(root, DEV, saved("d1", chars, a, r), now).unwrap();
        }
        // The last batch waits until the project closes.
        assert_eq!(values(root, DEV).len(), 2);
        flush(root);
        let lines = values(root, DEV);
        assert_eq!(kinds_of(&lines), ["save", "save", "save"]);
        let sum = |key: &str| lines.iter().map(|v| v[key].as_u64().unwrap()).sum::<u64>();
        assert_eq!(sum("saves"), 100);
        assert_eq!((sum("added"), sum("removed")), (added, removed));
        // A batch is closed by the save ten minutes after its first.
        assert_eq!(lines[0]["saves"], 41);
        assert_eq!(lines[0]["since"], "2026-10-02T01:00:00.000Z");
        assert_eq!(lines[0]["time"], "2026-10-02T01:10:00.000Z");
        assert_eq!(lines[1]["since"], "2026-10-02T01:10:15.000Z");
        let last = &lines[2];
        assert_eq!(last["time"], "2026-10-02T01:24:45.000Z");
        assert_eq!(last["chars"], u64::from(chars));
        assert_eq!(last["body"], fingerprint(format!("d1{chars}").as_bytes()));
        assert!(verify(root).unwrap().ok);
        let s = summary(root, Some(DEV)).unwrap();
        assert_eq!((s.saves, s.this_device), (100, 3));
    }

    #[test]
    fn a_waiting_batch_goes_in_before_any_other_line() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // January, so `flush_due` here never reaches other tests' batches.
        let t = |s: &str| at(&format!("2026-01-02T01:{s}.000Z"));
        note_at(root, DEV, saved("d1", 10, 10, 0), t("00:00")).unwrap();
        note_at(root, DEV, saved("d1", 30, 20, 0), t("00:30")).unwrap();
        // A record: the two saves first, then the record.
        let record = Entry::Snapshot(Snapshot {
            doc: "d1".into(),
            snapshot: "20261002-010100-000.manual".into(),
            body: fingerprint(b"d130"),
            snapshot_kind: "manual".into(),
        });
        note_at(root, DEV, record, t("01:00")).unwrap();
        // Another chapter saved: the first chapter's batch goes in.
        note_at(root, DEV, saved("d1", 25, 0, 5), t("02:00")).unwrap();
        note_at(root, DEV, saved("d2", 7, 7, 0), t("03:00")).unwrap();
        // The editor's session line (`append`) goes after the waiting save.
        let session = Entry::Session(Session {
            doc: "d2".into(),
            start: "2026-01-02T01:02:30.000Z".into(),
            end: "2026-01-02T01:03:30.000Z".into(),
            inserted: 7,
            deleted: 0,
        });
        append_at(root, DEV, &session, "2026-01-02T01:04:00.000Z").unwrap();
        let lines = values(root, DEV);
        assert_eq!(
            kinds_of(&lines),
            ["save", "snapshot", "save", "save", "session"]
        );
        assert_eq!(
            (lines[0]["saves"].as_u64(), lines[0]["added"].as_u64()),
            (Some(2), Some(30))
        );
        assert_eq!(lines[0]["time"], "2026-01-02T01:00:30.000Z");
        assert_eq!(
            (lines[2]["doc"].as_str(), lines[2]["removed"].as_u64()),
            (Some("d1"), Some(5))
        );
        assert_eq!(
            (lines[3]["doc"].as_str(), lines[3]["saves"].as_u64()),
            (Some("d2"), Some(1))
        );
        assert!(verify(root).unwrap().ok);

        // Ten minutes after its first save, the app's minute check writes it.
        note_at(root, DEV, saved("d2", 9, 2, 0), t("05:00")).unwrap();
        flush_due(t("14:59"));
        assert_eq!(values(root, DEV).len(), 5);
        flush_due(t("15:00"));
        assert_eq!(values(root, DEV).len(), 6);
        assert!(verify(root).unwrap().ok);
    }

    #[test]
    fn months_are_pieces_of_one_chain() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        append_at(root, DEV, &save("d1", 1), "2026-09-30T23:59:00.000Z").unwrap();
        append_at(root, DEV, &save("d1", 2), "2026-10-01T00:01:00.000Z").unwrap();
        append_at(root, DEV, &save("d1", 3), "2026-10-15T00:00:00.000Z").unwrap();
        let sept = piece_file(root, DEV, "2026-09");
        let oct = piece_file(root, DEV, "2026-10");
        assert_eq!(pieces(root, DEV).unwrap(), [sept.clone(), oct.clone()]);
        let sept_text = fs::read_to_string(&sept).unwrap();
        let oct_text = fs::read_to_string(&oct).unwrap();
        assert_eq!(
            (sept_text.lines().count(), oct_text.lines().count()),
            (1, 2)
        );
        // October's first line links to September's last.
        let first_oct = oct_text.lines().next().unwrap();
        assert!(first_oct.ends_with(&format!(
            ",\"prev\":\"{}\"}}",
            fingerprint(sept_text.lines().next().unwrap().as_bytes())
        )));
        let report = verify(root).unwrap();
        assert!(report.ok);
        assert_eq!((report.files.len(), report.files[0].lines), (1, 3));

        // A clock put back never writes into a past month.
        append_at(root, DEV, &save("d1", 4), "2026-09-20T00:00:00.000Z").unwrap();
        assert_eq!(fs::read_to_string(&sept).unwrap(), sept_text);
        assert_eq!(fs::read_to_string(&oct).unwrap().lines().count(), 3);
        assert!(verify(root).unwrap().ok);

        // Removing a past month shows where the chain breaks.
        fs::remove_file(&sept).unwrap();
        assert_eq!(
            verify(root).unwrap().files[0].first_bad,
            Some((1, Problem::Broken))
        );
    }

    #[test]
    fn the_old_single_file_is_the_first_piece() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // Written by a version before monthly pieces, the last line cut short.
        let first = encode(&save("d1", 1), "2026-08-01T00:00:00.000Z", NO_PREV);
        let torn = "{\"kind\":\"sa";
        let single = single_file(root, DEV);
        fs::create_dir_all(single.parent().unwrap()).unwrap();
        let old = format!("{first}\n{torn}");
        fs::write(&single, &old).unwrap();

        append_at(root, DEV, &save("d1", 2), "2026-10-01T00:00:00.000Z").unwrap();
        // The old file is never written again; the chain goes on from it.
        assert_eq!(fs::read_to_string(&single).unwrap(), old);
        let oct = fs::read_to_string(piece_file(root, DEV, "2026-10")).unwrap();
        assert!(oct.ends_with(&format!(
            ",\"prev\":\"{}\"}}\n",
            fingerprint(torn.as_bytes())
        )));
        let lines = read_device(root, DEV).unwrap();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[1], torn.as_bytes());
        // The cut line shows, as before.
        assert_eq!(
            verify(root).unwrap().files[0].first_bad,
            Some((2, Problem::Unreadable))
        );

        // Without the cut line, old file and pieces check out as one.
        let dir2 = tempfile::tempdir().unwrap();
        let root2 = dir2.path();
        let single2 = single_file(root2, DEV);
        fs::create_dir_all(single2.parent().unwrap()).unwrap();
        fs::write(&single2, format!("{first}\n")).unwrap();
        append_at(root2, DEV, &save("d1", 2), "2026-10-01T00:00:00.000Z").unwrap();
        append_at(root2, DEV, &save("d1", 3), "2026-11-01T00:00:00.000Z").unwrap();
        // Another device's journal from before pieces, not written since.
        let other = encode(&save("d1", 4), "2026-08-02T00:00:00.000Z", NO_PREV);
        fs::write(single_file(root2, "dev2bbbbbbbb"), format!("{other}\n")).unwrap();
        let report = verify(root2).unwrap();
        assert!(report.ok, "{report:?}");
        assert_eq!(report.files.len(), 2);
        assert_eq!(report.files[0].lines, 3);
        let s = summary(root2, Some(DEV)).unwrap();
        assert_eq!((s.saves, s.devices, s.this_device), (4, 2, 3));
        assert_eq!(s.since.as_deref(), Some("2026-08-01T00:00:00.000Z"));
        // The time stamps' folder is never taken for a device.
        fs::create_dir_all(root2.join(JOURNAL_DIR).join(ANCHOR_DIR)).unwrap();
        assert_eq!(read_all(root2).unwrap().len(), 2);
    }
}
