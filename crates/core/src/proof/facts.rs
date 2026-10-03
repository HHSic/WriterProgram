//! What the certificate says, worked out from the journals, the chapters
//! and their records: daily amounts, a row per chapter, imports, exchanges
//! and the one-sentence summary (§5.2).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::time::{Duration, Instant};

use chrono::{DateTime, FixedOffset, NaiveDate, Utc};
use serde_json::Value;
use similar::{Algorithm, DiffTag, capture_diff_slices_deadline};

use super::Options;
use crate::count::count_blocks;
use crate::doc::{self, DocFile};
use crate::journal::{fingerprint, saves_of};
use crate::markup::write_body;
use crate::project::{self, ProjectKind};
use crate::store::parse_iso;
use crate::{Error, Result, corrections, snapshot};

/// The chapters and period a certificate covers.
#[derive(Debug, Clone)]
pub(crate) struct Scope {
    /// Chapters covered; none for the whole work.
    pub docs: Option<HashSet<String>>,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
    /// The writer's time zone, for "which day".
    pub tz: FixedOffset,
}

fn day(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d")
        .map_err(|_| Error::Invalid(format!("올바르지 않은 날짜: {s}")))
}

impl Scope {
    pub fn new(options: &Options, tz: FixedOffset) -> Result<Scope> {
        let from = options.from.as_deref().map(day).transpose()?;
        let to = options.to.as_deref().map(day).transpose()?;
        if let (Some(f), Some(t)) = (from, to)
            && f > t
        {
            return Err(Error::Invalid("기간의 시작이 끝보다 늦음".into()));
        }
        let docs = options
            .docs
            .as_ref()
            .map(|d| d.iter().cloned().collect::<HashSet<_>>());
        if docs.as_ref().is_some_and(HashSet::is_empty) {
            return Err(Error::Invalid("증명서에 넣을 회차를 골라 주세요".into()));
        }
        Ok(Scope { docs, from, to, tz })
    }

    pub fn date(&self, t: DateTime<Utc>) -> NaiveDate {
        t.with_timezone(&self.tz).date_naive()
    }

    pub fn in_period(&self, d: NaiveDate) -> bool {
        self.from.is_none_or(|f| d >= f) && self.to.is_none_or(|t| d <= t)
    }

    pub fn has_doc(&self, id: &str) -> bool {
        self.docs.as_ref().is_none_or(|d| d.contains(id))
    }

    /// A journal line is about the chapters covered (anchor lines are about
    /// everything; lines of kinds this version does not know, about nothing).
    pub fn covers(&self, v: &Value) -> bool {
        match v.get("kind").and_then(Value::as_str) {
            Some("anchor") => true,
            Some("import") => v
                .get("docs")
                .and_then(Value::as_array)
                .is_some_and(|d| d.iter().filter_map(Value::as_str).any(|d| self.has_doc(d))),
            Some(_) => v
                .get("doc")
                .and_then(Value::as_str)
                .is_some_and(|d| self.has_doc(d)),
            None => false,
        }
    }
}

/// One readable journal line.
#[derive(Debug, Clone)]
pub(crate) struct Event {
    pub time: DateTime<Utc>,
    pub date: NaiveDate,
    pub kind: String,
    pub v: Value,
}

impl Event {
    pub fn parse(line: &[u8], scope: &Scope) -> Option<Event> {
        let v: Value = serde_json::from_slice(line).ok()?;
        let kind = v.get("kind")?.as_str()?.to_string();
        let time = parse_iso(v.get("time")?.as_str()?)?;
        Some(Event {
            date: scope.date(time),
            time,
            kind,
            v,
        })
    }

    fn str(&self, key: &str) -> &str {
        self.v.get(key).and_then(Value::as_str).unwrap_or("")
    }

    fn num(&self, key: &str) -> u64 {
        self.v.get(key).and_then(Value::as_u64).unwrap_or(0)
    }
}

/// A manuscript chapter as it is now.
#[derive(Debug, Clone)]
pub(crate) struct Chapter {
    pub id: String,
    /// Place in the whole manuscript, from 1.
    pub number: usize,
    pub title: String,
    /// SHA-256 (hex) of the body, as in the journal and the anchors.
    pub body: String,
    pub chars: u32,
    pub file: DocFile,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Day {
    pub inserted: u64,
    pub deleted: u64,
    pub sessions: u64,
    pub saves: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Row {
    pub number: usize,
    pub title: String,
    pub chars: u32,
    pub first: Option<NaiveDate>,
    pub last_edit: Option<NaiveDate>,
    pub sessions: u64,
    pub inserted: u64,
    pub deleted: u64,
    pub outside_pastes: u64,
    pub outside_chars: u64,
    /// How much of the text changed since its oldest record (0–1).
    pub changed: Option<f64>,
    pub imported_from: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ImportRow {
    pub time: DateTime<Utc>,
    pub file: String,
    pub hash: String,
    pub numbers: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ExchangeRow {
    pub time: DateTime<Utc>,
    pub files: Vec<String>,
    /// (chapter number, fingerprint of the text sent)
    pub sent: Vec<(usize, String)>,
    /// (when, file name, fingerprint)
    pub received: Vec<(DateTime<Utc>, String, String)>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Totals {
    pub first: Option<NaiveDate>,
    pub last: Option<NaiveDate>,
    pub active_days: usize,
    pub sessions: u64,
    pub saves: u64,
    pub inserted: u64,
    pub deleted: u64,
    pub outside_chars: u64,
    pub outside_pastes: u64,
    pub inside_pastes: u64,
    pub devices: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct Facts {
    pub title: String,
    pub pen_name: String,
    pub kind: ProjectKind,
    pub chapters: Vec<Chapter>,
    pub days: BTreeMap<NaiveDate, Day>,
    pub rows: Vec<Row>,
    pub imports: Vec<ImportRow>,
    pub exchanges: Vec<ExchangeRow>,
    pub totals: Totals,
}

/// "화" or "장": what one chapter is called with its number.
pub(crate) fn noun(kind: ProjectKind) -> &'static str {
    match kind {
        ProjectKind::Webnovel => "화",
        ProjectKind::Print => "장",
    }
}

/// The manuscript chapters in order, all of them.
pub(crate) fn chapters(root: &Path) -> Result<(project::Project, Vec<Chapter>)> {
    let project = project::load(root)?;
    let mut out = Vec::new();
    for (i, id) in project.parts.iter().flat_map(|p| &p.docs).enumerate() {
        let Ok(file) = doc::load(root, id) else {
            continue;
        };
        out.push(Chapter {
            id: id.clone(),
            number: i + 1,
            title: file.meta.title.clone(),
            body: fingerprint(write_body(&file.body).as_bytes()),
            chars: count_blocks(&file.body).with_spaces,
            file,
        });
    }
    Ok((project, out))
}

/// Works out the facts for `scope` from every device's journal lines.
pub(crate) fn gather(
    root: &Path,
    scope: &Scope,
    journals: &[(String, Vec<Vec<u8>>)],
) -> Result<Facts> {
    let (project, all) = chapters(root)?;
    let number: HashMap<&str, usize> = all.iter().map(|c| (c.id.as_str(), c.number)).collect();
    let chapters: Vec<Chapter> = all
        .iter()
        .filter(|c| scope.has_doc(&c.id))
        .cloned()
        .collect();
    if let Some(docs) = &scope.docs
        && let Some(missing) = docs.iter().find(|d| !number.contains_key(d.as_str()))
    {
        return Err(Error::NotFound(format!("회차를 찾을 수 없음: {missing}")));
    }

    let mut events: Vec<Event> = Vec::new();
    let mut devices = 0;
    for (_, lines) in journals {
        let before = events.len();
        events.extend(
            lines
                .iter()
                .filter_map(|l| Event::parse(l, scope))
                .filter(|e| e.kind != "anchor" && scope.in_period(e.date) && scope.covers(&e.v)),
        );
        if events.len() > before {
            devices += 1;
        }
    }
    events.sort_by_key(|e| e.time);

    let mut days: BTreeMap<NaiveDate, Day> = BTreeMap::new();
    let mut rows: HashMap<&str, Row> = chapters
        .iter()
        .map(|c| {
            let row = Row {
                number: c.number,
                title: c.title.clone(),
                chars: c.chars,
                ..Row::default()
            };
            (c.id.as_str(), row)
        })
        .collect();
    let mut totals = Totals {
        devices,
        ..Totals::default()
    };
    let mut imports = Vec::new();
    for e in &events {
        let doc = e.str("doc");
        if let Some(row) = rows.get_mut(doc) {
            row.first = Some(row.first.map_or(e.date, |f| f.min(e.date)));
        }
        match e.kind.as_str() {
            "session" => {
                let (ins, del) = (e.num("inserted"), e.num("deleted"));
                let d = days.entry(e.date).or_default();
                d.inserted += ins;
                d.deleted += del;
                d.sessions += 1;
                totals.sessions += 1;
                totals.inserted += ins;
                totals.deleted += del;
                if let Some(row) = rows.get_mut(doc) {
                    row.sessions += 1;
                    row.inserted += ins;
                    row.deleted += del;
                }
            }
            "save" => {
                // A gathered line stands for several saves.
                let saves = saves_of(&e.v);
                days.entry(e.date).or_default().saves += saves;
                totals.saves += saves;
                if let Some(row) = rows.get_mut(doc) {
                    row.last_edit = Some(e.date);
                }
            }
            "paste" => {
                if e.v.get("outside").and_then(Value::as_bool) == Some(true) {
                    totals.outside_pastes += 1;
                    totals.outside_chars += e.num("chars");
                    if let Some(row) = rows.get_mut(doc) {
                        row.outside_pastes += 1;
                        row.outside_chars += e.num("chars");
                    }
                } else {
                    totals.inside_pastes += 1;
                }
            }
            "import" => {
                let docs: Vec<&str> =
                    e.v.get("docs")
                        .and_then(Value::as_array)
                        .map(|d| d.iter().filter_map(Value::as_str).collect())
                        .unwrap_or_default();
                let mut numbers: Vec<usize> = docs
                    .iter()
                    .filter(|d| scope.has_doc(d))
                    .filter_map(|d| number.get(d).copied())
                    .collect();
                numbers.sort_unstable();
                for d in &docs {
                    if let Some(row) = rows.get_mut(d) {
                        row.imported_from
                            .get_or_insert_with(|| e.str("file").into());
                        row.first = Some(row.first.map_or(e.date, |f| f.min(e.date)));
                    }
                }
                imports.push(ImportRow {
                    time: e.time,
                    file: e.str("file").into(),
                    hash: e.str("fileHash").into(),
                    numbers,
                });
            }
            _ => {}
        }
    }
    // Journals from before writing sessions were kept: count saves instead.
    if totals.sessions == 0 {
        for e in events.iter().filter(|e| e.kind == "save") {
            let d = days.entry(e.date).or_default();
            d.inserted += e.num("added");
            d.deleted += e.num("removed");
            totals.inserted += e.num("added");
            totals.deleted += e.num("removed");
        }
    }
    let active: Vec<NaiveDate> = days
        .iter()
        .filter(|(_, d)| d.sessions + d.saves > 0)
        .map(|(date, _)| *date)
        .collect();
    totals.first = active.first().copied();
    totals.last = active.last().copied();
    totals.active_days = active.len();

    let deadline = Instant::now() + Duration::from_secs(3);
    let rows = chapters
        .iter()
        .map(|c| {
            let mut row = rows.remove(c.id.as_str()).unwrap_or_default();
            row.changed = draft_of(root, &c.id)
                .ok()
                .flatten()
                .map(|draft| changed(&draft, &c.file, deadline));
            row
        })
        .collect();

    Ok(Facts {
        title: project.title.clone(),
        pen_name: project.pen_name.clone(),
        kind: project.kind,
        exchanges: exchanges(root, scope, &number)?,
        chapters,
        days,
        rows,
        imports,
        totals,
    })
}

/// The oldest record of a chapter: its draft, as far as the records go.
pub(crate) fn draft_of(root: &Path, id: &str) -> Result<Option<DocFile>> {
    let list = snapshot::list(root, id)?;
    match list.last() {
        Some(oldest) => snapshot::load(root, id, &oldest.id).map(Some),
        None => Ok(None),
    }
}

/// The share of characters that differ between two texts (0 same, 1 all new).
pub(crate) fn changed(a: &DocFile, b: &DocFile, deadline: Instant) -> f64 {
    let a: Vec<char> = write_body(&a.body).chars().collect();
    let b: Vec<char> = write_body(&b.body).chars().collect();
    if a.is_empty() && b.is_empty() {
        return 0.0;
    }
    let same: usize = capture_diff_slices_deadline(Algorithm::Myers, &a, &b, Some(deadline))
        .iter()
        .map(|op| op.as_tag_tuple())
        .filter(|(tag, _, _)| *tag == DiffTag::Equal)
        .map(|(_, r, _)| r.len())
        .sum();
    1.0 - (2 * same) as f64 / (a.len() + b.len()) as f64
}

fn exchanges(
    root: &Path,
    scope: &Scope,
    number: &HashMap<&str, usize>,
) -> Result<Vec<ExchangeRow>> {
    let mut out = Vec::new();
    for info in corrections::list(root)? {
        let ex = info.exchange;
        let Some(time) = parse_iso(&ex.created) else {
            continue;
        };
        let sent: Vec<(usize, String)> = ex
            .chapters
            .iter()
            .filter(|c| scope.has_doc(&c.doc_id))
            .filter_map(|c| Some((*number.get(c.doc_id.as_str())?, c.fingerprint.clone())))
            .collect();
        if sent.is_empty() || !scope.in_period(scope.date(time)) {
            continue;
        }
        let received = ex
            .received
            .iter()
            .filter_map(|r| Some((parse_iso(&r.at)?, r.name.clone(), r.fingerprint.clone())))
            .collect();
        out.push(ExchangeRow {
            time,
            files: ex.files.clone(),
            sent,
            received,
        });
    }
    out.sort_by_key(|r| r.time);
    Ok(out)
}

/// "1~3, 5, 7~9화"
pub(crate) fn ranges(numbers: &[usize], noun: &str) -> String {
    let mut sorted = numbers.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    let mut parts: Vec<String> = Vec::new();
    let mut i = 0;
    while i < sorted.len() {
        let start = sorted[i];
        let mut end = start;
        while i + 1 < sorted.len() && sorted[i + 1] == end + 1 {
            i += 1;
            end = sorted[i];
        }
        parts.push(if start == end {
            start.to_string()
        } else {
            format!("{start}~{end}")
        });
        i += 1;
    }
    format!("{}{noun}", parts.join(", "))
}

/// The topic particle after `word`: 은 after a final consonant, 는 after a
/// vowel, 은(는) when it is not Hangul.
pub(crate) fn topic(word: &str) -> &'static str {
    match word.chars().last() {
        Some(c @ '가'..='힣') => {
            if (c as u32 - '가' as u32).is_multiple_of(28) {
                "는"
            } else {
                "은"
            }
        }
        _ => "은(는)",
    }
}

/// "2026년 9월 1일"
pub(crate) fn date_text(d: NaiveDate) -> String {
    use chrono::Datelike;
    format!("{}년 {}월 {}일", d.year(), d.month(), d.day())
}

/// "1,234"
pub(crate) fn num(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// The disclaimer every certificate ends its summary with (§2).
pub const NOT_ABOUT_AI: &str = "이 증명서는 창작 과정의 기록이며 AI 사용 여부를 판단하지 않습니다.";

/// The summary sentence (§5.2 1).
pub(crate) fn summary(f: &Facts) -> String {
    let noun = noun(f.kind);
    let numbers: Vec<usize> = f.chapters.iter().map(|c| c.number).collect();
    let what = if numbers.is_empty() {
        format!("「{}」", f.title)
    } else {
        format!("「{}」 {}", f.title, ranges(&numbers, noun))
    };
    let t = &f.totals;
    let (Some(first), Some(last)) = (t.first, t.last) else {
        return format!(
            "{what}{} 이 기간에 남은 창작 일지 기록이 없습니다. {NOT_ABOUT_AI}",
            topic(&what)
        );
    };
    let times = if t.sessions > 0 { t.sessions } else { t.saves };
    let span = if first == last {
        format!("{} 하루 동안", date_text(first))
    } else {
        format!(
            "{}부터 {}까지 {}일 동안",
            date_text(first),
            date_text(last),
            num(t.active_days as u64)
        )
    };
    let mut text = format!(
        "{what}{} {span} {}번에 걸쳐 쓰고 고쳤습니다.",
        topic(&what),
        num(times)
    );

    let paste = if t.outside_chars == 0 {
        None
    } else {
        let share = t.outside_chars as f64 * 100.0 / t.inserted.max(t.outside_chars) as f64;
        Some(if share < 1.0 {
            "1% 미만".to_string()
        } else {
            format!("{}%", share.round() as u64)
        })
    };
    let mut by_file: Vec<(String, Vec<usize>)> = Vec::new();
    for import in f.imports.iter().filter(|i| !i.numbers.is_empty()) {
        match by_file.iter_mut().find(|(file, _)| *file == import.file) {
            Some((_, n)) => n.extend(&import.numbers),
            None => by_file.push((import.file.clone(), import.numbers.clone())),
        }
    }
    let imported = by_file
        .iter()
        .map(|(file, n)| {
            let chapters = ranges(n, noun);
            format!("{chapters}{} '{file}'에서", topic(&chapters))
        })
        .collect::<Vec<_>>()
        .join(", ");
    let rest = match (paste, imported.is_empty()) {
        (Some(p), false) => {
            format!(" 바깥에서 붙여 넣은 글은 전체의 {p}이고, {imported} 가져왔습니다.")
        }
        (Some(p), true) => format!(" 바깥에서 붙여 넣은 글은 전체의 {p}입니다."),
        (None, false) => format!(" 바깥에서 붙여 넣은 글은 없고, {imported} 가져왔습니다."),
        (None, true) => " 바깥에서 붙여 넣은 글은 없습니다.".to_string(),
    };
    text.push_str(&rest);
    text.push(' ');
    text.push_str(NOT_ABOUT_AI);
    text
}
