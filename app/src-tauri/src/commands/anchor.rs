//! Time anchoring (시각 고정): sends one 32-byte fingerprint to the public
//! time-stamping authorities and keeps their signed replies
//! (`writer_core::anchor`). The only network code of the creation proof;
//! nothing happens unless the writer allowed it on this device.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, State};
use writer_core::anchor::{self, Occasion, Pending, Skip, TSAS, Tsa};
use writer_core::journal;

use crate::error::{Res, fail};
use crate::paths::journal_file;
use crate::state::AppState;

/// A reply is a few kilobytes; anything far larger is not one.
const MAX_REPLY: u64 = 256 * 1024;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnchorResult {
    /// `signed`, `ask` (one is due and the writer wants to be asked),
    /// `notAllowed` (the writer has not allowed it), `journalOff`,
    /// `noJournal`, `unchanged` (nothing new since the last stamp), `notYet`
    /// (too little written since today's), `enough` (today's limit reached)
    /// or `offline` (no authority answered: try again later).
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
fn send(
    root: PathBuf,
    device: String,
    occasion: Occasion,
    ask_first: bool,
    wait: Duration,
) -> Res<Sent> {
    let pending = match anchor::prepare(&root, &device, occasion).map_err(fail)? {
        Ok(pending) => pending,
        Err(skip) => return Ok(Sent::Skipped(skip)),
    };
    if ask_first {
        return Ok(Sent::Ask);
    }
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(wait))
        .http_status_as_error(false)
        .user_agent("WriterProgram")
        .build()
        .into();
    let request = pending.request();
    // All authorities at once, so waiting is as long as the slowest one, not
    // the sum (it matters when the app is closing).
    let replies = std::thread::scope(|scope| {
        let asks: Vec<_> = TSAS
            .iter()
            .map(|tsa| {
                let (agent, request) = (&agent, &request);
                scope.spawn(move || (*tsa, ask(agent, tsa, request)))
            })
            .collect();
        asks.into_iter()
            .map(|a| a.join().expect("asking an authority does not panic"))
            .collect()
    });
    Ok(Sent::Replies(pending, replies))
}

/// What working out a stamp came to, before it is kept.
enum Sent {
    Skipped(Skip),
    /// One is due, and the writer wants to be asked first.
    Ask,
    Replies(Pending, Replies),
}

/// Stamps `root` when one is due for `occasion` (`check`, `moment`,
/// `closing` or `now`; see `anchor::Occasion`). With 물어보고 받기 on, a due
/// stamp comes back as `ask` unless the writer asked (`now`). When closing
/// the authorities get a few seconds only, so the window is not held up.
/// Failing to reach them is not an error; the screen tries again later.
/// Writing never waits for this.
#[tauri::command]
pub async fn journal_anchor(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
    occasion: String,
) -> Res<AnchorResult> {
    let settings = journal::load_settings(&journal_file(&app)?).map_err(fail)?;
    if settings.anchor != Some(true) {
        return Ok(AnchorResult::only("notAllowed"));
    }
    let Some(device) = journal::device() else {
        return Ok(AnchorResult::only("journalOff"));
    };
    let (occasion, wait) = match occasion.as_str() {
        "now" => (Occasion::Now, Duration::from_secs(20)),
        "moment" => (Occasion::Moment, Duration::from_secs(20)),
        "closing" => (Occasion::Moment, Duration::from_secs(5)),
        _ => (Occasion::Check, Duration::from_secs(20)),
    };
    let ask_first = settings.anchor_ask && occasion != Occasion::Now;
    let path = PathBuf::from(&root);
    let sent =
        tauri::async_runtime::spawn_blocking(move || send(path, device, occasion, ask_first, wait))
            .await
            .map_err(|e| e.to_string())??;
    let (pending, replies) = match sent {
        Sent::Replies(pending, replies) => (pending, replies),
        Sent::Ask => return Ok(AnchorResult::only("ask")),
        Sent::Skipped(Skip::NoJournal) => return Ok(AnchorResult::only("noJournal")),
        Sent::Skipped(Skip::Unchanged) => return Ok(AnchorResult::only("unchanged")),
        Sent::Skipped(Skip::NotYet) => return Ok(AnchorResult::only("notYet")),
        Sent::Skipped(Skip::Enough) => return Ok(AnchorResult::only("enough")),
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
