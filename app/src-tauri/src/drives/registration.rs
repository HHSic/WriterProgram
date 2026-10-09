//! Registration settings entered in the connection wizard, kept only on this device.

use std::path::Path;
use std::sync::atomic::Ordering;

use serde_json::Value;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;
use writer_core::store::atomic_write;
use writer_sync::accounts::Registry;
use writer_sync::providers::{AppId, AppIds, Provider};

use super::connect::DriveState;
use crate::error::{Res, fail};
use crate::paths::{apps_file, registry_file};

#[tauri::command]
pub async fn drive_registration_get(app: AppHandle, provider: Provider) -> Res<Option<AppId>> {
    let file = apps_file(&app)?;
    read_registration_file(&file)?;
    Ok(AppIds::load(&file).get(provider).cloned())
}

#[tauri::command]
pub async fn drive_registration_save(
    app: AppHandle,
    drives: State<'_, DriveState>,
    provider: Provider,
    registration: AppId,
) -> Res<()> {
    let _lock = drives
        .settings
        .lock()
        .map_err(|_| "드라이브 설정을 열지 못함")?;
    if drives.connecting.load(Ordering::Relaxed) {
        return Err("로그인이 끝나거나 취소된 뒤 등록 정보를 바꿔 주세요.".into());
    }
    if Registry::load(&registry_file(&app)?)
        .connection(provider)
        .is_some()
    {
        return Err("이 드라이브의 연결을 끊은 뒤 등록 정보를 바꿔 주세요.".into());
    }
    save_registration(&apps_file(&app)?, provider, registration)
}

fn save_registration(file: &Path, provider: Provider, mut registration: AppId) -> Res<()> {
    registration.client_id = registration.client_id.trim().into();
    registration.client_secret = registration
        .client_secret
        .map(|secret| secret.trim().to_string())
        .filter(|secret| !secret.is_empty());
    let valid = |text: &str| {
        !text.is_empty()
            && text.len() <= 2048
            && !text.chars().any(|c| c.is_whitespace() || c.is_control())
    };
    if !valid(&registration.client_id) {
        return Err("앱 ID를 공백 없이 올바르게 입력해 주세요.".into());
    }
    if provider == Provider::Google && !registration.client_secret.as_deref().is_some_and(valid) {
        return Err("Google 클라이언트 보안 비밀을 입력해 주세요.".into());
    }
    if registration.redirect_port == Some(0) {
        return Err("돌아오는 주소의 포트가 올바르지 않습니다.".into());
    }
    if provider != Provider::Google {
        registration.client_secret = None;
    }
    // Read strictly: a damaged existing file must never silently erase another drive's settings.
    let mut value = read_registration_file(file)?;
    let object = value
        .as_object_mut()
        .ok_or("드라이브 등록 파일 형식이 올바르지 않음")?;
    object.insert(
        provider.key().into(),
        serde_json::to_value(registration).map_err(|_| "등록 정보를 저장하지 못함")?,
    );
    let bytes = serde_json::to_vec_pretty(&value).map_err(|_| "등록 정보를 저장하지 못함")?;
    atomic_write(file, &bytes).map_err(fail)
}

fn read_registration_file(file: &Path) -> Res<Value> {
    match std::fs::read(file) {
        Ok(bytes) => {
            serde_json::from_slice::<AppIds>(&bytes)
                .map_err(|_| "기존 드라이브 등록 파일을 읽지 못했습니다. 파일을 확인해 주세요.")?;
            serde_json::from_slice::<Value>(&bytes)
                .map_err(|_| "드라이브 등록 파일을 읽지 못함".into())
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::json!({})),
        Err(_) => {
            Err("드라이브 등록 파일을 읽지 못했습니다. 파일 접근 권한을 확인해 주세요.".into())
        }
    }
}

/// Open the provider's console in the system browser, including on the start screen.
#[tauri::command]
pub async fn drive_registration_open(app: AppHandle, provider: Provider) -> Res<()> {
    let url = match provider {
        Provider::Google => "https://console.cloud.google.com/",
        Provider::Onedrive => "https://entra.microsoft.com/",
        Provider::Dropbox => "https://www.dropbox.com/developers/apps",
    };
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|_| "등록 페이지를 열지 못했습니다. 브라우저를 확인해 주세요.".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registration(id: &str) -> AppId {
        AppId {
            client_id: id.into(),
            client_secret: None,
            redirect_port: None,
        }
    }

    #[test]
    fn saving_one_drive_preserves_other_drives_and_extensions() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("settings/drive-apps.json");
        save_registration(&file, Provider::Dropbox, registration("dropbox-app")).unwrap();
        let mut value: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        value["custom"] = serde_json::json!({"keep": true});
        atomic_write(&file, &serde_json::to_vec(&value).unwrap()).unwrap();
        let before = value["dropbox"].clone();
        save_registration(&file, Provider::Onedrive, registration("  microsoft-app  ")).unwrap();
        let saved: Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        assert_eq!(saved["dropbox"], before);
        assert_eq!(saved["custom"], value["custom"]);
        assert_eq!(
            AppIds::load(&file)
                .get(Provider::Onedrive)
                .unwrap()
                .client_id,
            "microsoft-app"
        );
    }

    #[test]
    fn invalid_input_and_damaged_files_are_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("drive-apps.json");
        save_registration(&file, Provider::Dropbox, registration("original")).unwrap();
        let original = std::fs::read(&file).unwrap();
        for (provider, input) in [
            (Provider::Dropbox, "bad id"),
            (Provider::Google, "missing-secret"),
        ] {
            assert!(save_registration(&file, provider, registration(input)).is_err());
            assert_eq!(std::fs::read(&file).unwrap(), original);
        }
        std::fs::write(&file, b"{broken").unwrap();
        assert!(read_registration_file(&file).is_err());
        assert!(save_registration(&file, Provider::Onedrive, registration("new-app")).is_err());
        assert_eq!(std::fs::read(&file).unwrap(), b"{broken");
    }

    #[test]
    fn google_secret_and_custom_redirect_port_survive_restart() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("drive-apps.json");
        let mut input = registration(" google.apps.googleusercontent.com ");
        input.client_secret = Some(" secret ".into());
        input.redirect_port = Some(45678);
        save_registration(&file, Provider::Google, input).unwrap();
        let ids = AppIds::load(&file);
        let saved = ids.get(Provider::Google).unwrap();
        assert_eq!(saved.client_secret.as_deref(), Some("secret"));
        assert_eq!(saved.redirect_port, Some(45678));
    }
}
