//! Creation journal (창작 일지): a quiet record of how a work was written, so
//! the writer can later show it (docs/creation-proof.md).
//!
//! Each device appends to its own file, `.journal/<device id>.jsonl`, one JSON
//! object per line:
//!
//! ```text
//! {"kind":"save","time":"2026-10-02T01:02:03.456Z","doc":"k7q2m9x4t1ab","body":"9f2c…","chars":5120,"added":42,"removed":0,"prev":"3b1a…"}
//! ```
//!
//! `prev` is the SHA-256 (hex) of the previous line's bytes, or 64 zeros on
//! the first line: a hash chain, so changing or removing a line in the middle
//! shows up in `verify`. Lines are only ever added; the app never rewrites or
//! removes them. Two devices never write the same file, so a sync program
//! carrying the folder between them has nothing to merge.
//!
//! No manuscript text goes in: only ids, fingerprints, counts and times.
//!
//! The app keeps this device's id and the on/off switch in its settings
//! folder (`Settings`) and turns the journal on by telling this module the
//! device id (`set_device`); with none set, the hooks in `doc`, `snapshot`,
//! `import` and `corrections` write nothing.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::copies::is_id;
use crate::store::{atomic_write, new_id, now_iso, parse_iso, sha256, to_iso};
use crate::{Error, Result};

pub const JOURNAL_DIR: &str = ".journal";
const EXTENSION: &str = "jsonl";
/// `prev` of the first line of a file.
const NO_PREV: &str = "0000000000000000000000000000000000000000000000000000000000000000";
/// Pastes shorter than this are not worth a line (spec §3).
pub const PASTE_MIN_CHARS: u32 = 100;

/// The device the journal is written for; none while the journal is off.
static DEVICE: RwLock<Option<String>> = RwLock::new(None);
/// Appends one at a time, so two lines never chain to the same previous one.
static APPEND: Mutex<()> = Mutex::new(());

/// Turns the journal on for `device` (its id, see `new_device_id`), or off
/// with none. Called by the app at start and when the writer switches it.
pub fn set_device(device: Option<String>) {
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
    // Later: `anchor` (time-stamping, §4.2).
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
    /// How much longer or shorter the text got with this save.
    pub added: u32,
    pub removed: u32,
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

impl Entry {
    pub fn kind(&self) -> &'static str {
        match self {
            Entry::Save(_) => "save",
            Entry::Session(_) => "session",
            Entry::Paste(_) => "paste",
            Entry::Import(_) => "import",
            Entry::Snapshot(_) => "snapshot",
            Entry::Exchange(_) => "exchange",
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

fn file_of(root: &Path, device: &str) -> PathBuf {
    root.join(JOURNAL_DIR).join(format!("{device}.{EXTENSION}"))
}

/// Adds `entry` to this device's journal in `root`.
pub fn append(root: &Path, device: &str, entry: &Entry) -> Result<()> {
    append_at(root, device, entry, &now_iso())
}

fn append_at(root: &Path, device: &str, entry: &Entry, time: &str) -> Result<()> {
    if !is_id(device) {
        return Err(Error::Invalid("올바르지 않은 기기 이름".into()));
    }
    let _one = APPEND.lock().unwrap_or_else(|p| p.into_inner());
    let path = file_of(root, device);
    let dir = path.parent().expect("inside the journal folder");
    fs::create_dir_all(dir).map_err(|e| Error::io(dir, e))?;
    let mut file = open_retry(&path).map_err(|e| Error::io(&path, e))?;
    let (last, torn) = last_line(&mut file).map_err(|e| Error::io(&path, e))?;
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

/// Adds `entry` to the journal when it is on; called from saves, records and
/// imports. A journal that cannot be written never stops the work itself.
pub(crate) fn note(root: &Path, entry: Entry) {
    if let Some(device) = device()
        && let Err(e) = append(root, &device, &entry)
    {
        eprintln!("창작 일지를 쓰지 못함: {}", e.user_message());
    }
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

/// The journal files in `root`: (device id, path), sorted by device.
fn files(root: &Path) -> Result<Vec<(String, PathBuf)>> {
    let dir = root.join(JOURNAL_DIR);
    let entries = match fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(Error::io(&dir, e)),
    };
    let mut out: Vec<(String, PathBuf)> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                return None;
            }
            let device = name.strip_suffix(&format!(".{EXTENSION}"))?.to_string();
            Some((device, e.path()))
        })
        .collect();
    out.sort();
    Ok(out)
}

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

fn check_file(device: &str, bytes: &[u8]) -> FileCheck {
    let lines = lines(bytes);
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

/// Checks the chain of every device's journal in `root`.
pub fn verify(root: &Path) -> Result<Report> {
    let mut checks = Vec::new();
    for (device, path) in files(root)? {
        let bytes = fs::read(&path).map_err(|e| Error::io(&path, e))?;
        checks.push(check_file(&device, &bytes));
    }
    let ok = checks.iter().all(|c| c.first_bad.is_none());
    Ok(Report { files: checks, ok })
}

/// What the journal holds, for the writer to see that it works.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    /// Time of the earliest line on any device.
    pub since: Option<String>,
    /// Saves on all devices.
    pub saves: usize,
    pub sessions: usize,
    pub pastes: usize,
    pub imports: usize,
    /// 교정 주고받기 lines: chapters sent, files taken back, corrections applied.
    pub exchanges: usize,
    /// Devices with a journal in this project.
    pub devices: usize,
    /// Lines written by this device.
    pub this_device: usize,
}

pub fn summary(root: &Path, device: Option<&str>) -> Result<Summary> {
    let mut out = Summary::default();
    let mut since: Option<chrono::DateTime<chrono::Utc>> = None;
    for (name, path) in files(root)? {
        let bytes = fs::read(&path).map_err(|e| Error::io(&path, e))?;
        out.devices += 1;
        let lines = lines(&bytes);
        if device == Some(name.as_str()) {
            out.this_device = lines.len();
        }
        for line in lines {
            let Ok(v) = serde_json::from_slice::<Value>(line) else {
                continue;
            };
            match v.get("kind").and_then(Value::as_str) {
                Some("save") => out.saves += 1,
                Some("session") => out.sessions += 1,
                Some("paste") => out.pastes += 1,
                Some("import") => out.imports += 1,
                Some("exchange") => out.exchanges += 1,
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
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn save(doc: &str, chars: u32) -> Entry {
        Entry::Save(Save {
            doc: doc.into(),
            body: fingerprint(doc.as_bytes()),
            chars,
            added: chars,
            removed: 0,
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
        let text = fs::read_to_string(root.join(".journal/dev1aaaaaaaa.jsonl")).unwrap();
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
        let path = root.join(".journal/dev1aaaaaaaa.jsonl");
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
        let path = root.join(".journal/dev1aaaaaaaa.jsonl");
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
        let text = fs::read_to_string(root.join(".journal/dev1aaaaaaaa.jsonl")).unwrap();
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
        let text = fs::read_to_string(dir.path().join(".journal/dev1aaaaaaaa.jsonl")).unwrap();
        assert!(text.starts_with(
            "{\"kind\":\"save\",\"time\":\"2026-10-02T01:02:03.456Z\",\"doc\":\"d1\","
        ));
    }
}
