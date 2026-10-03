//! New versions of the app (tauri-plugin-updater): asking GitHub Releases
//! whether there is one, then fetching, checking and installing it. The
//! package is signed with the project's own key; the plugin refuses one whose
//! signature does not match the public key in tauri.conf.json.
//!
//! The screen saves everything before it asks for the install: on Windows
//! the installer closes the app and opens the new version.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::error::Res;

/// The update found by the last check, kept for the install.
#[derive(Default)]
pub struct PendingUpdate(Mutex<Option<Update>>);

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub current: String,
    /// What changed, as written in the release.
    pub notes: Option<String>,
    /// When the release was made (RFC 3339).
    pub date: Option<String>,
}

/// How far the download has come, for the screen ("update-progress").
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    received: u64,
    total: Option<u64>,
}

fn reason(e: tauri_plugin_updater::Error) -> String {
    use tauri_plugin_updater::Error as E;
    match e {
        E::Reqwest(e) if e.is_timeout() => {
            "시간 안에 받지 못함 · 인터넷 연결을 확인해 주세요".into()
        }
        E::Reqwest(e) if e.is_connect() => "인터넷에 연결되지 않아 새 버전을 확인하지 못함".into(),
        E::Reqwest(e) => format!("새 버전을 받지 못함 · {e}"),
        E::Network(e) => format!("새 버전을 받지 못함 · {e}"),
        E::Minisign(_) | E::SignatureUtf8(_) | E::Base64(_) => {
            "받은 파일의 서명이 맞지 않아 설치하지 않음".into()
        }
        other => format!("새 버전을 확인하지 못함 · {other}"),
    }
}

/// The app's version, for 보기 설정.
#[tauri::command]
pub fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Asks whether a newer version is out. `WRITER_UPDATE_ENDPOINT` points the
/// check elsewhere, for trying it without a release.
#[tauri::command]
pub async fn update_check(
    app: AppHandle,
    pending: State<'_, PendingUpdate>,
) -> Res<Option<UpdateInfo>> {
    // A limit on connecting only: a whole-request timeout would also cut off
    // the download of a large installer on a slow line.
    let mut builder = app
        .updater_builder()
        .configure_client(|client| client.connect_timeout(Duration::from_secs(15)));
    if let Ok(url) = std::env::var("WRITER_UPDATE_ENDPOINT") {
        let url = url
            .parse()
            .map_err(|_| "확인할 주소가 올바르지 않음".to_string())?;
        builder = builder.endpoints(vec![url]).map_err(reason)?;
    }
    let found = builder
        .build()
        .map_err(reason)?
        .check()
        .await
        .map_err(reason)?;
    let info = found.as_ref().map(|u| UpdateInfo {
        version: u.version.clone(),
        current: u.current_version.clone(),
        notes: u.body.clone().filter(|b| !b.trim().is_empty()),
        date: u.date.and_then(|d| {
            d.format(&time::format_description::well_known::Rfc3339)
                .ok()
        }),
    });
    *pending.0.lock().map_err(|e| e.to_string())? = found;
    Ok(info)
}

/// Fetches the update found by the last check, checks its signature and
/// installs it. On Windows the installer then closes this app and opens the
/// new version. With `WRITER_UPDATE_DRY` set it stops after the check of the
/// signature (for trying it without installing anything).
#[tauri::command]
pub async fn update_install(app: AppHandle, pending: State<'_, PendingUpdate>) -> Res<()> {
    let update = pending
        .0
        .lock()
        .map_err(|e| e.to_string())?
        .take()
        .ok_or_else(|| "먼저 새 버전을 확인해 주세요".to_string())?;
    let mut received: u64 = 0;
    // Tell the screen only when the share moves by a percent, not per chunk.
    let mut told: Option<u64> = None;
    let bytes = update
        .download(
            |chunk, total| {
                received += chunk as u64;
                let step = total.map(|t| received * 100 / t.max(1));
                if step != told || total.is_none() && received % (1 << 20) < chunk as u64 {
                    told = step;
                    let _ = app.emit("update-progress", Progress { received, total });
                }
            },
            || {},
        )
        .await
        .map_err(reason)?;
    if std::env::var_os("WRITER_UPDATE_DRY").is_some() {
        return Ok(());
    }
    update.install(bytes).map_err(reason)?;
    // Windows has left by now; elsewhere the new version needs a restart.
    app.restart();
}
