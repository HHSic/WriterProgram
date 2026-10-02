//! AI with the writer's own API key (자기 API 키 연결): this device's choices,
//! the key in the system's credential store, and the two kinds of help
//! (회차 요약, 설정 모순 점검). See `writer_ai`.
//!
//! The key is read from the credential store only to make a call and is
//! never written to a file, a log or an answer to the screen. The text sent
//! and the answers live only in the screen's memory.

use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, State};
use writer_ai::client::Client;
use writer_ai::provider::{ALL, clean_key};
use writer_ai::settings::{self, Models, Patch, Settings};
use writer_ai::tasks::{self, Check, Preview, Summary, Task};
use writer_ai::{Error, Provider};
use writer_sync::secrets::Secrets;

use crate::drives::{blocking, secrets};
use crate::error::{Res, fail};
use crate::paths::ai_file;
use crate::state::AppState;

/// One company as the settings screen shows it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Company {
    provider: Provider,
    label: &'static str,
    models: Models,
    defaults: Models,
    /// A key for it is in the credential store (the key itself never leaves).
    has_key: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiSettings {
    enabled: bool,
    provider: Provider,
    agreed: Option<String>,
    companies: Vec<Company>,
}

/// Where a company's key is kept in the credential store.
fn key_name(provider: Provider) -> String {
    format!("ai/{}", provider.key())
}

fn view(settings: &Settings, secrets: &dyn Secrets) -> Res<AiSettings> {
    let mut companies = Vec::new();
    for provider in ALL {
        companies.push(Company {
            provider,
            label: provider.label(),
            models: settings.models(provider),
            defaults: Models::defaults(provider),
            has_key: secrets.get(&key_name(provider)).map_err(fail)?.is_some(),
        });
    }
    Ok(AiSettings {
        enabled: settings.enabled,
        provider: settings.provider,
        agreed: settings.agreed.clone(),
        companies,
    })
}

fn put_key(secrets: &dyn Secrets, provider: Provider, key: &str) -> Res<()> {
    let key = clean_key(key).map_err(fail)?;
    secrets.set(&key_name(provider), &key).map_err(fail)
}

fn key_of(secrets: &dyn Secrets, provider: Provider) -> Res<String> {
    secrets
        .get(&key_name(provider))
        .map_err(fail)?
        .ok_or_else(|| fail(Error::NoKey))
}

/// The client and the two models, when AI is on and a key is there.
fn ready(path: &Path, secrets: &dyn Secrets) -> Res<(Client, Models)> {
    let settings = settings::load(path);
    if !settings.enabled {
        return Err(fail(Error::Off));
    }
    let provider = settings.provider;
    let key = key_of(secrets, provider)?;
    Ok((
        Client::new(provider, &key, provider.base()),
        settings.models(provider),
    ))
}

#[tauri::command]
pub async fn ai_settings(app: AppHandle) -> Res<AiSettings> {
    view(&settings::load(&ai_file(&app)?), secrets().as_ref())
}

/// Turns AI on or off (on notes when the writer agreed), picks the company
/// or changes a company's models.
#[tauri::command]
pub async fn ai_settings_set(
    app: AppHandle,
    state: State<'_, AppState>,
    patch: Patch,
) -> Res<AiSettings> {
    let _write = state.write();
    let path = ai_file(&app)?;
    let mut settings = settings::load(&path);
    settings.apply(patch).map_err(fail)?;
    settings::save(&path, &settings).map_err(fail)?;
    view(&settings, secrets().as_ref())
}

#[tauri::command]
pub async fn ai_key_set(app: AppHandle, provider: Provider, key: String) -> Res<AiSettings> {
    let secrets = secrets();
    put_key(secrets.as_ref(), provider, &key)?;
    view(&settings::load(&ai_file(&app)?), secrets.as_ref())
}

#[tauri::command]
pub async fn ai_key_remove(app: AppHandle, provider: Provider) -> Res<AiSettings> {
    let secrets = secrets();
    secrets.delete(&key_name(provider)).map_err(fail)?;
    view(&settings::load(&ai_file(&app)?), secrets.as_ref())
}

/// 연결 확인: asks the chosen company about the two models with the key.
/// Sends no manuscript and costs nothing; works before AI is turned on.
#[tauri::command]
pub async fn ai_connection_check(app: AppHandle) -> Res<String> {
    let path = ai_file(&app)?;
    blocking(move || {
        let settings = settings::load(&path);
        let provider = settings.provider;
        let key = key_of(secrets().as_ref(), provider)?;
        let client = Client::new(provider, &key, provider.base());
        let models = settings.models(provider);
        client.check_model(&models.summary).map_err(fail)?;
        if models.check != models.summary {
            client.check_model(&models.check).map_err(fail)?;
        }
        Ok(format!(
            "{}에 연결됨. 요약에 {}, 점검에 {}을(를) 씁니다.",
            provider.label(),
            models.summary,
            models.check
        ))
    })
    .await
}

/// What would be sent for `task` on each chapter, before anything is sent.
#[tauri::command]
pub async fn ai_preview(
    app: AppHandle,
    root: String,
    task: Task,
    doc_ids: Vec<String>,
) -> Res<Vec<Preview>> {
    let path = ai_file(&app)?;
    blocking(move || {
        ready(&path, secrets().as_ref())?;
        let root = PathBuf::from(root);
        doc_ids
            .iter()
            .map(|id| {
                tasks::prepare(&root, task, id)
                    .map(|out| out.preview())
                    .map_err(fail)
            })
            .collect()
    })
    .await
}

/// 회차 요약 of one chapter. The answer is shown; nothing is changed.
#[tauri::command]
pub async fn ai_summarize(app: AppHandle, root: String, doc_id: String) -> Res<Summary> {
    let path = ai_file(&app)?;
    blocking(move || {
        let (client, models) = ready(&path, secrets().as_ref())?;
        let out = tasks::prepare(Path::new(&root), Task::Summary, &doc_id).map_err(fail)?;
        let answer = client.ask(&models.summary, &out.ask).map_err(fail)?;
        tasks::read_summary(&out, answer).map_err(fail)
    })
    .await
}

/// 설정 모순 점검 of one chapter against its setting cards.
#[tauri::command]
pub async fn ai_check(app: AppHandle, root: String, doc_id: String) -> Res<Check> {
    let path = ai_file(&app)?;
    blocking(move || {
        let (client, models) = ready(&path, secrets().as_ref())?;
        let out = tasks::prepare(Path::new(&root), Task::Check, &doc_id).map_err(fail)?;
        let answer = client.ask(&models.check, &out.ask).map_err(fail)?;
        tasks::read_check(&out, answer).map_err(fail)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use writer_sync::secrets::MemorySecrets;

    #[test]
    fn keys_stay_in_the_store() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ai.json");
        let secrets = MemorySecrets::default();

        // Off by default: nothing can be sent.
        assert!(ready(&path, &secrets).err().unwrap().contains("꺼져 있음"));

        put_key(&secrets, Provider::Gemini, "  AIza-secret \n").unwrap();
        assert_eq!(
            secrets.get("ai/gemini").unwrap().as_deref(),
            Some("AIza-secret")
        );
        assert!(put_key(&secrets, Provider::Gemini, "two words").is_err());

        let mut s = settings::load(&path);
        s.apply(Patch {
            enabled: Some(true),
            provider: Some(Provider::Anthropic),
            models: None,
        })
        .unwrap();
        settings::save(&path, &s).unwrap();
        // On, but no key for the chosen company.
        assert!(
            ready(&path, &secrets)
                .err()
                .unwrap()
                .contains("API 키가 없음")
        );

        let shown = view(&s, &secrets).unwrap();
        let has: Vec<bool> = shown.companies.iter().map(|c| c.has_key).collect();
        assert_eq!(has, vec![false, false, true]);
        // Neither the screen's view nor the settings file holds the key.
        let json = serde_json::to_string(&shown).unwrap();
        assert!(!json.contains("AIza-secret"));
        assert!(!std::fs::read_to_string(&path).unwrap().contains("AIza"));

        put_key(&secrets, Provider::Anthropic, "sk-ant").unwrap();
        assert!(ready(&path, &secrets).is_ok());
    }
}
