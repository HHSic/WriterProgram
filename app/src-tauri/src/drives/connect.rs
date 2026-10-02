//! Connecting a drive: which drives can sign in and who is signed in,
//! signing in through the drive's page, and forgetting a drive.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use writer_sync::accounts::{self, Registry};
use writer_sync::providers::{Account, AppIds, Endpoints, Provider};

use super::{blocking, secrets};
use crate::error::{Res, fail};
use crate::paths::{apps_file, registry_file};

#[derive(Default)]
pub struct DriveState {
    /// Set to stop a sign-in waiting for the browser.
    cancel: Arc<AtomicBool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
    provider: Provider,
    label: &'static str,
    /// The app is registered with this drive (it can sign in).
    registered: bool,
    account: Option<Account>,
    connected_at: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveStatus {
    drives: Vec<DriveInfo>,
    /// Where registrations can be put (drive-apps.json).
    apps_file: String,
}

#[tauri::command]
pub async fn drive_status(app: AppHandle) -> Res<DriveStatus> {
    let registry = Registry::load(&registry_file(&app)?);
    let ids = AppIds::load(&apps_file(&app)?);
    Ok(DriveStatus {
        drives: Provider::ALL
            .iter()
            .map(|&p| {
                let connection = registry.connection(p);
                DriveInfo {
                    provider: p,
                    label: p.label(),
                    registered: ids.get(p).is_some(),
                    account: connection.map(|c| c.account.clone()),
                    connected_at: connection.map(|c| c.connected_at.clone()),
                }
            })
            .collect(),
        apps_file: apps_file(&app)?.to_string_lossy().into_owned(),
    })
}

/// Opens the drive's sign-in page and waits for the writer to come back.
#[tauri::command]
pub async fn drive_connect(
    app: AppHandle,
    drives: State<'_, DriveState>,
    provider: Provider,
) -> Res<Account> {
    let ids = AppIds::load(&apps_file(&app)?);
    let id = ids
        .get(provider)
        .cloned()
        .ok_or_else(|| format!("{} 앱 등록 전이라 아직 연결할 수 없음", provider.label()))?;
    let cancel = drives.cancel.clone();
    cancel.store(false, Ordering::Relaxed);
    blocking(move || {
        let opener = app.clone();
        let connection = accounts::sign_in(
            provider,
            &id,
            Endpoints::of(provider),
            secrets(),
            move |url| open_sign_in(&opener, url),
            &cancel,
        )
        .map_err(fail)?;
        let file = registry_file(&app)?;
        let mut registry = Registry::load(&file);
        if let Some(old) = registry.connection(provider)
            && old.key != connection.key
        {
            let _ = secrets().delete(&old.key);
        }
        registry.connections.retain(|c| c.provider != provider);
        let account = connection.account.clone();
        registry.connections.push(connection);
        registry.save(&file).map_err(fail)?;
        Ok(account)
    })
    .await
}

/// The sign-in page in the writer's browser. For automated checks against a
/// stand-in drive, `WRITER_SIGN_IN=fetch` has the app visit it itself.
fn open_sign_in(app: &AppHandle, url: &str) -> writer_sync::Result<()> {
    if std::env::var("WRITER_SIGN_IN").as_deref() == Ok("fetch") {
        let url = url.to_string();
        // The stand-in answers the sign-in page with a redirect back to the app.
        std::thread::spawn(move || {
            let _ = writer_sync::http::fetch(&url);
        });
        return Ok(());
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| writer_sync::Error::Invalid(format!("브라우저를 열지 못함 ({e})")))
}

#[tauri::command]
pub async fn drive_cancel(drives: State<'_, DriveState>) -> Res<()> {
    drives.cancel.store(true, Ordering::Relaxed);
    Ok(())
}

/// Forgets a drive on this device: its sign-in, and which projects followed it.
#[tauri::command]
pub async fn drive_disconnect(app: AppHandle, provider: Provider) -> Res<()> {
    let file = registry_file(&app)?;
    let mut registry = Registry::load(&file);
    if let Some(connection) = registry.connection(provider) {
        secrets().delete(&connection.key).map_err(fail)?;
    }
    registry.connections.retain(|c| c.provider != provider);
    registry.links.retain(|_, l| l.provider != provider);
    registry.save(&file).map_err(fail)
}
