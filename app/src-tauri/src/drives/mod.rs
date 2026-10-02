//! 기기 간 맞추기 with the writer's own drive (writer_sync): signing in,
//! which projects follow which drive folder, and running passes.
//!
//! What is kept where on this device:
//! - the sign-ins (refresh tokens): the system's credential store
//!   (Windows 자격 증명 관리자), through keyring;
//! - which drives are connected and which projects follow them:
//!   `drives.json` in the app's settings folder;
//! - what each project looked like after its last pass: `sync/<id>.json`;
//! - the app's registrations with the drives: `drive-apps.json` (or built in).
//!
//! `keyring` keeps the sign-ins, `connect` signs in and out, `projects`
//! links projects to drive folders and runs the passes.

use std::sync::Arc;

use tauri::AppHandle;
use writer_sync::accounts::Registry;
use writer_sync::providers::{AppIds, Endpoints, Provider, Session};
use writer_sync::secrets::Secrets;

use crate::error::{Res, fail};
use crate::paths::apps_file;

pub mod connect;
pub mod keyring;
pub mod projects;

pub(crate) fn secrets() -> Arc<dyn Secrets> {
    Arc::new(keyring::KeyringSecrets)
}

/// Runs blocking work (network, files) away from the screen's thread.
pub(crate) async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> Res<T> + Send + 'static,
) -> Res<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| e.to_string())?
}

fn session(app: &AppHandle, registry: &Registry, provider: Provider) -> Res<Session> {
    let ids = AppIds::load(&apps_file(app)?);
    let id = ids
        .get(provider)
        .ok_or_else(|| format!("{} 앱 등록이 없어 연결할 수 없음", provider.label()))?;
    let connection = registry
        .connection(provider)
        .ok_or_else(|| format!("{}에 연결되어 있지 않음", provider.label()))?;
    Session::resume(
        provider,
        id,
        Endpoints::of(provider),
        secrets(),
        connection.key.clone(),
    )
    .map_err(fail)
}
