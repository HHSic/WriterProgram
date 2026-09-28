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

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;
use writer_core::project::{self, Overview};
use writer_core::store::now_iso;
use writer_sync::accounts::{self, Link, Registry};
use writer_sync::engine::Report;
use writer_sync::providers::{self, Account, AppIds, Endpoints, Provider, Session};
use writer_sync::remote::Remote;
use writer_sync::secrets::Secrets;

use crate::commands::{AppState, recent_file};

type Res<T> = Result<T, String>;

fn fail(e: writer_sync::Error) -> String {
    e.user_message()
}

/// Sign-ins in the system's credential store. A value longer than one
/// credential can hold (Windows: 2,560 bytes) is split over several.
pub struct KeyringSecrets;

const SERVICE: &str = "WriterProgram";
/// Characters per credential (stored as UTF-16).
const CHUNK: usize = 1000;

fn entry(key: &str) -> writer_sync::Result<keyring_core::Entry> {
    keyring_core::Entry::new(SERVICE, key)
        .map_err(|e| writer_sync::Error::Invalid(format!("자격 증명 저장소를 쓰지 못함 ({e})")))
}

fn store_err(e: keyring_core::Error) -> writer_sync::Error {
    writer_sync::Error::Invalid(format!("자격 증명 저장소를 쓰지 못함 ({e})"))
}

impl Secrets for KeyringSecrets {
    fn get(&self, key: &str) -> writer_sync::Result<Option<String>> {
        let head = match entry(key)?.get_password() {
            Ok(v) => v,
            Err(keyring_core::Error::NoEntry) => return Ok(None),
            Err(e) => return Err(store_err(e)),
        };
        let Some(count) = head
            .strip_prefix("chunks:")
            .and_then(|n| n.parse::<usize>().ok())
        else {
            return Ok(Some(head));
        };
        let mut value = String::new();
        for i in 1..=count {
            value.push_str(
                &entry(&format!("{key}#{i}"))?
                    .get_password()
                    .map_err(store_err)?,
            );
        }
        Ok(Some(value))
    }

    fn set(&self, key: &str, value: &str) -> writer_sync::Result<()> {
        self.delete(key)?;
        let chars: Vec<char> = value.chars().collect();
        if chars.len() <= CHUNK {
            return entry(key)?.set_password(value).map_err(store_err);
        }
        let parts: Vec<String> = chars.chunks(CHUNK).map(|c| c.iter().collect()).collect();
        for (i, part) in parts.iter().enumerate() {
            entry(&format!("{key}#{}", i + 1))?
                .set_password(part)
                .map_err(store_err)?;
        }
        entry(key)?
            .set_password(&format!("chunks:{}", parts.len()))
            .map_err(store_err)
    }

    fn delete(&self, key: &str) -> writer_sync::Result<()> {
        let head = match entry(key)?.get_password() {
            Ok(v) => v,
            Err(keyring_core::Error::NoEntry) => return Ok(()),
            Err(e) => return Err(store_err(e)),
        };
        if let Some(count) = head
            .strip_prefix("chunks:")
            .and_then(|n| n.parse::<usize>().ok())
        {
            for i in 1..=count {
                let _ = entry(&format!("{key}#{i}"))?.delete_credential();
            }
        }
        match entry(key)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(e) => Err(store_err(e)),
        }
    }
}

/// Picks the system's credential store for keyring. Call once at start-up.
pub fn init_secrets() {
    #[cfg(windows)]
    if let Ok(store) = windows_native_keyring_store::Store::new() {
        keyring_core::set_default_store(store);
    }
}

#[derive(Default)]
pub struct DriveState {
    /// Set to stop a sign-in waiting for the browser.
    cancel: Arc<AtomicBool>,
}

fn dir(app: &AppHandle) -> Res<PathBuf> {
    app.path().app_config_dir().map_err(|e| e.to_string())
}

fn registry_file(app: &AppHandle) -> Res<PathBuf> {
    Ok(dir(app)?.join("drives.json"))
}

fn apps_file(app: &AppHandle) -> Res<PathBuf> {
    Ok(dir(app)?.join("drive-apps.json"))
}

fn base_dir(app: &AppHandle) -> Res<PathBuf> {
    Ok(dir(app)?.join("sync"))
}

fn secrets() -> Arc<dyn Secrets> {
    Arc::new(KeyringSecrets)
}

/// Runs blocking work (network, files) away from the screen's thread.
async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Res<T> + Send + 'static) -> Res<T> {
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
        std::thread::spawn(move || {
            let _ = ureq_get(&url);
        });
        return Ok(());
    }
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|e| writer_sync::Error::Invalid(format!("브라우저를 열지 못함 ({e})")))
}

fn ureq_get(url: &str) -> Result<(), String> {
    // The stand-in answers the sign-in page with a redirect back to the app.
    writer_sync::http::fetch(url).map_err(|e| e.to_string())
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

#[tauri::command]
pub async fn project_link_get(app: AppHandle, project_id: String) -> Res<Option<Link>> {
    Ok(Registry::load(&registry_file(&app)?)
        .links
        .get(&project_id)
        .cloned())
}

/// The project's id in a drive folder's project.json, if it has one.
fn folder_project(remote: &mut dyn Remote) -> writer_sync::Result<Option<(String, String)>> {
    if !remote
        .list()?
        .iter()
        .any(|f| f.path == project::PROJECT_FILE)
    {
        return Ok(None);
    }
    let bytes = remote.get(project::PROJECT_FILE)?;
    let p: project::Project = serde_json::from_slice(&bytes).map_err(|e| {
        writer_sync::Error::Drive(format!("드라이브의 작품 정보를 읽지 못함 ({e})"))
    })?;
    Ok(Some((p.id, p.title)))
}

/// Starts keeping a project in step with a folder on a drive: its own folder
/// when it is there already (from another device), else a new one.
#[tauri::command]
pub async fn project_link(
    app: AppHandle,
    project_id: String,
    title: String,
    provider: Provider,
) -> Res<Link> {
    blocking(move || {
        let file = registry_file(&app)?;
        let mut registry = Registry::load(&file);
        let session = session(&app, &registry, provider)?;
        let names = providers::projects(&session).map_err(fail)?;
        let base = providers::folder_name(&title);
        let mut folder = None;
        for n in 1..50 {
            let candidate = if n == 1 {
                base.clone()
            } else {
                format!("{base} ({n})")
            };
            if !names.contains(&candidate) {
                folder = Some(candidate);
                break;
            }
            let mut remote = providers::open(&session, &candidate, false).map_err(fail)?;
            match folder_project(remote.as_mut()).map_err(fail)? {
                Some((id, _)) if id != project_id => continue,
                _ => {
                    folder = Some(candidate);
                    break;
                }
            }
        }
        let folder = folder.ok_or_else(|| "드라이브에 쓸 폴더 이름을 정하지 못함".to_string())?;
        let link = Link {
            provider,
            folder,
            linked_at: now_iso(),
            synced_at: None,
            error: None,
        };
        registry.links.insert(project_id, link.clone());
        registry.save(&file).map_err(fail)?;
        Ok(link)
    })
    .await
}

#[tauri::command]
pub async fn project_unlink(app: AppHandle, project_id: String) -> Res<()> {
    let file = registry_file(&app)?;
    let mut registry = Registry::load(&file);
    registry.links.remove(&project_id);
    registry.save(&file).map_err(fail)?;
    let _ = std::fs::remove_file(accounts::base_file(&base_dir(&app)?, &project_id));
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncOutcome {
    link: Link,
    report: Option<Report>,
}

/// One pass for an open project. Errors end up in the link (and the report is
/// none) rather than failing, so the screen can say what happened.
#[tauri::command]
pub async fn project_sync(app: AppHandle, root: String, project_id: String) -> Res<SyncOutcome> {
    blocking(move || {
        let file = registry_file(&app)?;
        let registry = Registry::load(&file);
        let link = registry
            .links
            .get(&project_id)
            .cloned()
            .ok_or_else(|| "이 작품은 드라이브와 맞추지 않음".to_string())?;
        let result = session(&app, &registry, link.provider).and_then(|session| {
            let state = app.state::<AppState>();
            let base = accounts::base_file(&base_dir(&app)?, &project_id);
            accounts::sync_project(&session, &link, Path::new(&root), &base, state.lock())
                .map_err(fail)
        });
        // Read again: another pass or a link change may have happened meanwhile.
        let mut registry = Registry::load(&file);
        let Some(saved) = registry.links.get_mut(&project_id) else {
            return Err("이 작품은 드라이브와 맞추지 않음".into());
        };
        let report = match result {
            Ok(report) => {
                saved.synced_at = Some(now_iso());
                saved.error = None;
                Some(report)
            }
            Err(e) => {
                saved.error = Some(e);
                None
            }
        };
        let link = saved.clone();
        registry.save(&file).map_err(fail)?;
        Ok(SyncOutcome { link, report })
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveProject {
    folder: String,
    title: String,
    id: String,
}

/// Projects in a drive, to bring one to this device.
#[tauri::command]
pub async fn drive_projects(app: AppHandle, provider: Provider) -> Res<Vec<DriveProject>> {
    blocking(move || {
        let registry = Registry::load(&registry_file(&app)?);
        let session = session(&app, &registry, provider)?;
        let mut out = Vec::new();
        for folder in providers::projects(&session).map_err(fail)? {
            let mut remote = providers::open(&session, &folder, false).map_err(fail)?;
            if let Some((id, title)) = folder_project(remote.as_mut()).map_err(fail)? {
                out.push(DriveProject { folder, title, id });
            }
        }
        Ok(out)
    })
    .await
}

/// Brings a project from a drive into a new folder in `dest`, keeps it in step
/// from now on, and opens it.
#[tauri::command]
pub async fn drive_fetch(
    app: AppHandle,
    provider: Provider,
    folder: String,
    dest: String,
) -> Res<Overview> {
    blocking(move || {
        let file = registry_file(&app)?;
        let registry = Registry::load(&file);
        let session = session(&app, &registry, provider)?;
        let mut remote = providers::open(&session, &folder, false).map_err(fail)?;
        let (project_id, _) = folder_project(remote.as_mut())
            .map_err(fail)?
            .ok_or_else(|| "드라이브의 이 폴더에는 작품이 없음".to_string())?;
        drop(remote);
        let parent = PathBuf::from(&dest);
        let mut root = parent.join(&folder);
        let mut n = 2;
        while root.exists() {
            root = parent.join(format!("{folder} ({n})"));
            n += 1;
        }
        std::fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let link = Link {
            provider,
            folder,
            linked_at: now_iso(),
            synced_at: None,
            error: None,
        };
        let state = app.state::<AppState>();
        let base = accounts::base_file(&base_dir(&app)?, &project_id);
        let _ = std::fs::remove_file(&base);
        accounts::sync_project(&session, &link, &root, &base, state.lock()).map_err(fail)?;
        let overview = project::open(&root).map_err(|e| e.user_message())?;
        let _ = writer_core::recent::touch(&recent_file(&app)?, &overview);
        let mut registry = Registry::load(&file);
        registry.links.insert(
            project_id,
            Link {
                synced_at: Some(now_iso()),
                ..link
            },
        );
        registry.save(&file).map_err(fail)?;
        Ok(overview)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_sign_ins_are_split_and_joined() {
        keyring_core::set_default_store(keyring_core::mock::Store::new().unwrap());
        let s = KeyringSecrets;
        assert_eq!(s.get("t/short").unwrap(), None);
        s.set("t/short", "RT1").unwrap();
        assert_eq!(s.get("t/short").unwrap().as_deref(), Some("RT1"));

        // Microsoft's refresh tokens can be longer than one credential holds.
        let long: String = "가나다라마바사아자차카타파하0123456789".repeat(120);
        s.set("t/long", &long).unwrap();
        assert_eq!(s.get("t/long").unwrap().as_deref(), Some(long.as_str()));
        s.set("t/long", "short again").unwrap();
        assert_eq!(s.get("t/long").unwrap().as_deref(), Some("short again"));
        s.delete("t/long").unwrap();
        s.delete("t/short").unwrap();
        assert_eq!(s.get("t/long").unwrap(), None);
        assert_eq!(s.get("t/short").unwrap(), None);
    }
}
