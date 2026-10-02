//! Checking a proof file (§6), for anyone who receives one: the command-line
//! tool (`examples/verify_proof.rs`) and the certificate itself (made by
//! running this on the file it writes).
//!
//! 1. Every journal's hash chain links from its first line to its last
//!    (lines shown only as a hash link through their hash).
//! 2. Every stamp: its leaves give its root, each authority's reply is the
//!    file the record names, its signature checks out, and it signed that
//!    root. Each journal leaf must be a line of that journal: everything up
//!    to it existed by the time the authority signed.
//! 3. Times written in the journal agree with the stamps: a line before a
//!    stamped one should not claim a later time, and a line after it should
//!    not claim an earlier one (a device clock set back shows up here).
//! 4. Manuscript files given along: where their fingerprint appears.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;

use super::bundle::{Bundle, FORMAT, KIND, Line};
use crate::anchor::{merkle_root, tsa_label, tsp};
use crate::doc::parse_doc;
use crate::journal::{FIRST_PREV, fingerprint, hex};
use crate::markup::write_body;
use crate::store::parse_iso;

/// Clocks may differ this much from an authority's before a line is noted.
const CLOCK_SLACK: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// Checked and right.
    Ok,
    /// Worth knowing; not a fault.
    Note,
    /// Does not prove tampering, but needs a look.
    Warn,
    /// Does not check out.
    Fail,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub level: Level,
    pub text: String,
}

/// One stamp as checked.
#[derive(Debug, Clone)]
pub struct AnchorCheck {
    /// When it was asked for (device clock).
    pub time: String,
    pub root: String,
    /// Each authority (label) and what its reply came to.
    pub stamps: Vec<(String, Result<tsp::Stamp, String>)>,
    /// The earliest time an authority vouches for, when any reply checked out.
    pub signed: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct Verdict {
    /// Nothing failed.
    pub ok: bool,
    pub findings: Vec<Finding>,
    pub anchors: Vec<AnchorCheck>,
}

impl Verdict {
    /// The findings as lines of text, with a mark each.
    pub fn text(&self) -> String {
        let mut out = String::new();
        for f in &self.findings {
            let mark = match f.level {
                Level::Ok => "[확인]",
                Level::Note => "[참고]",
                Level::Warn => "[주의]",
                Level::Fail => "[실패]",
            };
            out.push_str(&format!("{mark} {}\n", f.text));
        }
        out.push_str(if self.ok {
            "\n결과: 이 증명 자료는 고쳐지지 않았고, 시각 인증이 맞습니다.\n"
        } else {
            "\n결과: 맞지 않는 곳이 있습니다. 위의 [실패]를 보세요.\n"
        });
        out
    }
}

struct Report {
    findings: Vec<Finding>,
}

impl Report {
    fn add(&mut self, level: Level, text: impl Into<String>) {
        self.findings.push(Finding {
            level,
            text: text.into(),
        });
    }
}

pub(crate) fn time_text(t: DateTime<Utc>) -> String {
    t.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Checks the proof file's bytes, with manuscript files (name, bytes).
pub fn verify(bundle: &[u8], files: &[(String, Vec<u8>)]) -> Verdict {
    match serde_json::from_slice::<Bundle>(bundle) {
        Ok(b) => check(&b, files),
        Err(e) => Verdict {
            ok: false,
            findings: vec![Finding {
                level: Level::Fail,
                text: format!("증명 자료 파일을 읽을 수 없습니다 ({e})."),
            }],
            anchors: Vec::new(),
        },
    }
}

/// A journal as followed: each line's hash, and the readable lines.
struct Followed {
    hashes: Vec<String>,
    times: Vec<Option<DateTime<Utc>>>,
    values: Vec<Option<Value>>,
}

fn follow(device: &str, lines: &[Line], r: &mut Report) -> Followed {
    let mut hashes: Vec<String> = Vec::with_capacity(lines.len());
    let mut times = Vec::with_capacity(lines.len());
    let mut values = Vec::with_capacity(lines.len());
    let mut broken = None;
    let mut shown = 0;
    for (i, line) in lines.iter().enumerate() {
        let prev = if i == 0 {
            FIRST_PREV.to_string()
        } else {
            hashes[i - 1].clone()
        };
        match line {
            Line::Shown(text) => {
                shown += 1;
                let v: Option<Value> = serde_json::from_str(text).ok();
                let link = v
                    .as_ref()
                    .and_then(|v| v.get("prev"))
                    .and_then(Value::as_str);
                if broken.is_none() {
                    if v.is_none() {
                        broken = Some((i, "읽을 수 없습니다"));
                    } else if link != Some(prev.as_str()) {
                        broken = Some((i, "앞 항목과 이어지지 않습니다"));
                    }
                }
                times.push(
                    v.as_ref()
                        .and_then(|v| v.get("time"))
                        .and_then(Value::as_str)
                        .and_then(parse_iso),
                );
                values.push(v);
            }
            Line::Hidden { hash } => {
                if broken.is_none()
                    && (hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()))
                {
                    broken = Some((i, "지문이 올바르지 않습니다"));
                }
                times.push(None);
                values.push(None);
            }
        }
        hashes.push(line.hash());
    }
    match broken {
        None => r.add(
            Level::Ok,
            format!(
                "기기 {device}의 창작 일지 {}개 항목(내용 공개 {shown}개)이 처음부터 끝까지 이어져 있습니다.",
                lines.len()
            ),
        ),
        Some((i, why)) => r.add(
            Level::Fail,
            format!(
                "기기 {device}의 창작 일지 {}번째 항목이 {why}. 그 앞이 고쳐졌거나 빠졌습니다.",
                i + 1
            ),
        ),
    }
    Followed {
        hashes,
        times,
        values,
    }
}

/// Checks a parsed proof file; see the module notes.
pub fn check(bundle: &Bundle, files: &[(String, Vec<u8>)]) -> Verdict {
    let mut r = Report {
        findings: Vec::new(),
    };
    if bundle.kind != KIND || bundle.format > FORMAT {
        r.add(
            Level::Fail,
            "이 검증 도구가 모르는 형식의 증명 자료입니다. 검증 도구를 새로 받아 주세요.",
        );
        return Verdict {
            ok: false,
            findings: r.findings,
            anchors: Vec::new(),
        };
    }
    r.add(
        Level::Note,
        format!(
            "「{}」의 증명 자료, {} 만듦.",
            bundle.title,
            parse_iso(&bundle.made).map_or(bundle.made.clone(), time_text)
        ),
    );

    // 1. Chains.
    let journals: HashMap<&str, Followed> = bundle
        .journals
        .iter()
        .map(|j| (j.device.as_str(), follow(&j.device, &j.lines, &mut r)))
        .collect();

    // 2. Stamps, and 3. times against them.
    let mut anchors = Vec::new();
    if bundle.anchors.is_empty() {
        r.add(
            Level::Note,
            "시각 인증이 없습니다. 기록이 이어져 있는지만 확인할 수 있습니다.",
        );
    }
    for part in &bundle.anchors {
        let record = &part.record;
        let asked = parse_iso(&record.time).map_or(record.time.clone(), time_text);
        let root = merkle_root(&record.leaves);
        if hex(&root) != record.root {
            r.add(
                Level::Fail,
                format!("{asked}의 시각 인증: 지문 목록으로 다시 계산한 지문이 기록과 다릅니다."),
            );
        }
        let mut stamps = Vec::new();
        for token in &part.tokens {
            let label = tsa_label(&token.tsa);
            let result = token
                .bytes()
                .ok_or_else(|| "토큰을 읽을 수 없음".to_string())
                .and_then(|bytes| {
                    let named = record.tokens.iter().find(|t| t.file == token.file);
                    if named.is_none_or(|t| t.sha256 != fingerprint(&bytes)) {
                        return Err("기록에 적힌 파일과 다름".to_string());
                    }
                    let stamp = tsp::check(&bytes)?;
                    if !stamp.sha256 || stamp.imprint != root {
                        return Err("다른 지문에 대한 시각 인증".to_string());
                    }
                    Ok(stamp)
                });
            match &result {
                Ok(s) => r.add(
                    Level::Ok,
                    format!(
                        "{label}가 {}에 이 지문({}…)에 서명했습니다. 서명 인증서: {}, 발급: {}.",
                        time_text(s.gen_time),
                        &record.root[..16],
                        s.signer,
                        s.top
                    ),
                ),
                Err(why) => r.add(Level::Fail, format!("{asked}의 시각 인증({label}): {why}.")),
            }
            stamps.push((label, result));
        }
        let signed = stamps
            .iter()
            .filter_map(|(_, s)| s.as_ref().ok())
            .map(|s| s.gen_time)
            .min();
        if part.tokens.is_empty() {
            r.add(
                Level::Warn,
                format!("{asked}의 시각 인증: 인증 기관의 응답 파일이 빠져 있습니다."),
            );
        }
        for leaf in &record.leaves {
            let Some(device) = leaf.device() else {
                continue;
            };
            let found = journals.get(device).and_then(|j| {
                j.hashes
                    .iter()
                    .position(|h| *h == leaf.value)
                    .map(|i| (j, i))
            });
            match (found, signed) {
                (None, _) => r.add(
                    Level::Fail,
                    format!("{asked}의 시각 인증이 가리키는 기기 {device}의 항목이 창작 일지에 없습니다."),
                ),
                (Some((j, i)), Some(signed)) => {
                    r.add(
                        Level::Ok,
                        format!(
                            "기기 {device}의 창작 일지 1~{}번째 항목은 늦어도 {}에 있었습니다.",
                            i + 1,
                            time_text(signed)
                        ),
                    );
                    clock_check(device, j, i, signed, &mut r);
                }
                (Some(_), None) => {}
            }
        }
        anchors.push(AnchorCheck {
            time: record.time.clone(),
            root: record.root.clone(),
            stamps,
            signed,
        });
    }

    // 4. Manuscript files.
    for (name, bytes) in files {
        manuscript(bundle, &journals, &anchors, name, bytes, &mut r);
    }

    let ok = r.findings.iter().all(|f| f.level != Level::Fail);
    Verdict {
        ok,
        findings: r.findings,
        anchors,
    }
}

/// Lines up to `at` should not claim times after `signed`; lines after it
/// not times before.
fn clock_check(device: &str, j: &Followed, at: usize, signed: DateTime<Utc>, r: &mut Report) {
    let slack = Duration::minutes(CLOCK_SLACK);
    let ahead = j.times[..=at]
        .iter()
        .flatten()
        .filter(|t| **t > signed + slack)
        .count();
    let behind = j.times[at + 1..]
        .iter()
        .flatten()
        .filter(|t| **t < signed - slack)
        .count();
    if ahead > 0 {
        r.add(
            Level::Warn,
            format!(
                "기기 {device}의 항목 {ahead}개가 시각 인증({})보다 늦은 시각을 적고 있습니다. 그 기기의 시계가 빨랐을 수 있습니다.",
                time_text(signed)
            ),
        );
    }
    if behind > 0 {
        r.add(
            Level::Warn,
            format!(
                "기기 {device}의 항목 {behind}개가 시각 인증({}) 뒤에 쓰였는데 그보다 이른 시각을 적고 있습니다. 그 기기의 시계가 늦었거나 날짜를 앞당겨 적은 것입니다.",
                time_text(signed)
            ),
        );
    }
}

/// Where a manuscript file's fingerprint appears.
fn manuscript(
    bundle: &Bundle,
    journals: &HashMap<&str, Followed>,
    anchors: &[AnchorCheck],
    name: &str,
    bytes: &[u8],
    r: &mut Report,
) {
    let raw = fingerprint(bytes);
    let body = std::str::from_utf8(bytes)
        .ok()
        .map(|text| fingerprint(write_body(&parse_doc(text, "").body).as_bytes()));
    let mut found = Vec::new();
    if let Some(body) = &body {
        if let Some(c) = bundle.chapters.iter().find(|c| c.body == *body) {
            found.push(format!(
                "증명서를 만들 때의 {}번째 회차와 같은 글입니다",
                c.number
            ));
        }
        let first_saved = journals
            .values()
            .flat_map(|j| j.values.iter().zip(&j.times))
            .filter_map(|(v, t)| Some((v.as_ref()?, (*t)?)))
            .filter(|(v, _)| v.get("body").and_then(Value::as_str) == Some(body.as_str()))
            .map(|(_, t)| t)
            .min();
        if let Some(t) = first_saved {
            found.push(format!(
                "창작 일지에 {} 저장으로 남아 있습니다",
                time_text(t)
            ));
        }
        let stamped = bundle
            .anchors
            .iter()
            .zip(anchors)
            .filter(|(part, _)| {
                part.record
                    .leaves
                    .iter()
                    .any(|l| l.doc_id().is_some() && l.value == *body)
            })
            .filter_map(|(_, check)| check.signed)
            .min();
        if let Some(t) = stamped {
            found.push(format!(
                "늦어도 {}에 있었음이 시각 인증으로 확인됩니다",
                time_text(t)
            ));
        }
    }
    let imported = journals
        .values()
        .flat_map(|j| &j.values)
        .flatten()
        .find(|v| v.get("fileHash").and_then(Value::as_str) == Some(raw.as_str()));
    if let Some(v) = imported {
        let file = v.get("file").and_then(Value::as_str).unwrap_or("");
        found.push(format!("가져온 파일 '{file}' 그대로입니다"));
    }
    if found.is_empty() {
        r.add(
            Level::Warn,
            format!("원고 파일 {name}의 지문을 증명 자료에서 찾지 못했습니다."),
        );
    } else {
        r.add(
            Level::Ok,
            format!("원고 파일 {name}: {}.", found.join(", ")),
        );
    }
}
