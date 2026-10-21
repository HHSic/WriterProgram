//! The kfiction check around the writing tools, in place of the Claude Code
//! hook, which does not see MCP writes (`plugins/kfiction/hooks`).
//!
//! Before saving, text with contract breaks (assistant talk, markdown) is
//! refused, so nothing half-done lands in the manuscript and a retried append
//! cannot double the text. Like the hook, a document is refused twice at most
//! within half an hour; the third try goes through and the answer lists what
//! is still wrong. After saving, the answer carries the chapter's warnings,
//! for the writer to decide on.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::check::{self, Severity};

pub const MAX_REFUSALS: u32 = 2;
const STALE: Duration = Duration::from_secs(30 * 60);
/// Warnings listed in an answer; the rest are counted.
const LISTED: usize = 20;

static REFUSED: LazyLock<Mutex<HashMap<String, (u32, Instant)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Checks `text` about to be written under `key` (project and document).
/// `Err` refuses it with the lines to fix; `Ok` lets it through, with the
/// contract breaks still in it once the refusals are used up.
pub fn before(key: &str, text: &str) -> Result<Vec<Value>, String> {
    let contract: Vec<_> = check::lint(text)
        .into_iter()
        .filter(|f| f.severity == Severity::Contract)
        .collect();
    let mut refused = REFUSED.lock().unwrap_or_else(|p| p.into_inner());
    if contract.is_empty() {
        refused.remove(key);
        return Ok(Vec::new());
    }
    let now = Instant::now();
    let count = match refused.get(key) {
        Some((n, at)) if now.duration_since(*at) < STALE => *n,
        _ => 0,
    };
    if count >= MAX_REFUSALS {
        refused.remove(key);
        return Ok(contract.iter().map(check::Finding::to_json).collect());
    }
    refused.insert(key.to_string(), (count + 1, now));
    let rows: Vec<String> = contract
        .iter()
        .map(|f| {
            format!(
                "L{} {} {}: \"{}\" -> {}",
                f.line.map_or("-".into(), |l| l.to_string()),
                f.rule,
                f.name,
                f.matched,
                f.hint
            )
        })
        .collect();
    Err(format!(
        "[kfiction] 보낸 글에 원고가 아닌 것이 섞여 저장하지 않았음 ({}/{}회차). 줄 번호는 보낸 글 기준.\n{}\n지적된 줄만 고쳐 같은 도구로 다시 보내세요. 설명·인사·강조 기호를 지우고 원고 문장은 그대로 둡니다.",
        count + 1,
        MAX_REFUSALS,
        rows.join("\n")
    ))
}

/// The chapter's warnings after a save, for the tool's answer.
pub fn after(chapter_text: &str, unfixed: Vec<Value>) -> Value {
    let found = check::lint(chapter_text);
    let warn = check::of(&found, Severity::Warn);
    let info = found
        .iter()
        .filter(|f| f.severity == Severity::Info)
        .count();
    let mut out = json!({
        "warn": warn.iter().take(LISTED).collect::<Vec<_>>(),
        "warn_count": warn.len(),
        "info_count": info,
        "note": "kfiction 점검. 경고는 참고용이라 작가가 고르게 한다. 작가가 원하지 않으면 고치지 않는다. 줄 번호는 read_doc 본문 기준.",
    });
    if !unfixed.is_empty() {
        out["unfixed_contract"] = Value::Array(unfixed);
        out["note"] = json!(
            "고치지 못한 계약 위반이 본문에 남았음(unfixed_contract). 작가에게 알린다. 경고는 참고용이다. 줄 번호는 read_doc 본문 기준."
        );
    }
    out
}

/// Every finding in `text`, for the `check` tool.
pub fn full(text: &str) -> Value {
    let found = check::lint(text);
    json!({
        "contract": check::of(&found, Severity::Contract),
        "warn": check::of(&found, Severity::Warn),
        "info": check::of(&found, Severity::Info),
        "out_of_scope": [
            "대사가 관계에 맞는 말 단계·호칭인지",
            "시점과 숨은 의도",
            "설정 모순 (이름, 시간, 능력, 세계 규칙)",
            "비유가 장면과 인물에 맞는지",
            "문장이 좋은지",
        ],
        "note": "막는 기준이 아니다. 사람이 쓴 소설도 1,000자당 두어 곳씩 걸린다. 지적된 곳만 보고 원고 전체를 다시 쓰지 않는다.",
    })
}
