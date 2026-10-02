//! Time anchoring (시각 고정): sends one 32-byte fingerprint to the public
//! time-stamping authorities and keeps their signed replies
//! (`writer_core::anchor`). The only network code of the creation proof;
//! nothing happens unless the writer allowed it on this device.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, State};
use writer_core::anchor::{self, Pending, Skip, TSAS, Tsa};
use writer_core::journal;

use crate::error::{Res, fail};
use crate::paths::journal_file;
use crate::state::AppState;

/// A reply is a few kilobytes; anything far larger is not one.
const MAX_REPLY: u64 = 256 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorResult {
    /// `signed`, `notAllowed` (the writer has not allowed it), `journalOff`,
    /// `noJournal`, `unchanged` (nothing new since the last stamp),
    /// `doneToday` or `offline` (no authority answered: try again later).
    state: &'static str,
    /// Authorities that signed.
    signed: Vec<String>,
}

impl AnchorResult {
    fn only(state: &'static str) -> Self {
        AnchorResult {
            state,
            signed: Vec::new(),
        }
    }
}

/// POSTs the request to one authority and returns its reply.
fn ask(agent: &ureq::Agent, tsa: &Tsa, request: &[u8]) -> Result<Vec<u8>, String> {
    let mut response = agent
        .post(tsa.url)
        .content_type("application/timestamp-query")
        .send(request)
        .map_err(|e| e.to_string())?;
    let status = response.status().as_u16();
    if status != 200 {
        return Err(format!("HTTP {status}"));
    }
    response
        .body_mut()
        .with_config()
        .limit(MAX_REPLY)
        .read_to_vec()
        .map_err(|e| e.to_string())
}

/// Each authority and its reply, or why there was none.
type Replies = Vec<(Tsa, Result<Vec<u8>, String>)>;

/// Works out what to send and asks every authority (off the screen's thread).
fn send(root: PathBuf, device: String, force: bool) -> Res<Result<(Pending, Replies), Skip>> {
    let pending = match anchor::prepare(&root, &device, force).map_err(fail)? {
        Ok(pending) => pending,
        Err(skip) => return Ok(Err(skip)),
    };
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(20)))
        .http_status_as_error(false)
        .user_agent("WriterProgram")
        .build()
        .into();
    let request = pending.request();
    let replies = TSAS
        .iter()
        .map(|tsa| (*tsa, ask(&agent, tsa, &request)))
        .collect();
    Ok(Ok((pending, replies)))
}

/// Stamps `root` if due: once a day when allowed, or now when the writer
/// asks (`force`). Failing to reach the authorities is not an error; the
/// screen tries again later. Writing never waits for this.
#[tauri::command]
pub async fn journal_anchor(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
    force: bool,
) -> Res<AnchorResult> {
    let settings = journal::load_settings(&journal_file(&app)?).map_err(fail)?;
    if settings.anchor != Some(true) {
        return Ok(AnchorResult::only("notAllowed"));
    }
    let Some(device) = journal::device() else {
        return Ok(AnchorResult::only("journalOff"));
    };
    let path = PathBuf::from(&root);
    let sent = tauri::async_runtime::spawn_blocking(move || send(path, device, force))
        .await
        .map_err(|e| e.to_string())??;
    let (pending, replies) = match sent {
        Ok(sent) => sent,
        Err(Skip::NoJournal) => return Ok(AnchorResult::only("noJournal")),
        Err(Skip::Unchanged) => return Ok(AnchorResult::only("unchanged")),
        Err(Skip::DoneToday) => return Ok(AnchorResult::only("doneToday")),
    };
    let outcome = {
        let _write = state.write();
        anchor::finish(Path::new(&root), &pending, replies).map_err(fail)?
    };
    for (tsa, why) in &outcome.failed {
        eprintln!("시각 인증을 받지 못함 ({tsa}): {why}");
    }
    Ok(if outcome.signed.is_empty() {
        AnchorResult::only("offline")
    } else {
        AnchorResult {
            state: "signed",
            signed: outcome.signed,
        }
    })
}
