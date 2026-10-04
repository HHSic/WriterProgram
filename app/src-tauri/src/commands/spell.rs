//! 맞춤법 검사 (`writer_core::spell`): fetching the dictionary once the
//! writer agrees (the only network use, a pinned and hash-checked GitHub
//! release), checking paragraphs for the editor and the whole work for the
//! 맞춤법 panel, suggestions, and the word lists (사용자 사전).
//!
//! The dictionary is loaded on first use and kept in `AppState`; checking
//! runs off the screen's thread.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, State};
use writer_core::spell::{self, DictInfo, Issue, Known, Speller, WorkCheck, install, words};

use crate::drives::blocking;
use crate::error::{Res, fail};
use crate::paths::{spelling_dir, spelling_words_file};
use crate::state::AppState;

/// The release is under a megabyte; refuse anything far larger.
const MAX_DOWNLOAD: u64 = 8 * 1024 * 1024;

const NO_DICTIONARY: &str = "맞춤법 사전이 없음";

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpellStatus {
    /// The dictionary on this device, if fetched.
    installed: Option<DictInfo>,
    /// What fetching it would bring.
    version: &'static str,
    download_bytes: u64,
    installed_bytes: u64,
    source: &'static str,
    notice: &'static str,
}

#[tauri::command]
pub async fn spell_status(app: AppHandle) -> Res<SpellStatus> {
    Ok(SpellStatus {
        installed: install::installed(&spelling_dir(&app)?),
        version: install::VERSION,
        download_bytes: install::DOWNLOAD_BYTES,
        installed_bytes: install::INSTALLED_BYTES,
        source: install::SOURCE,
        notice: install::NOTICE,
    })
}

fn fetch() -> Res<Vec<u8>> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(120)))
        .http_status_as_error(false)
        .user_agent("WriterProgram")
        .build()
        .into();
    let mut response = agent
        .get(install::URL)
        .call()
        .map_err(|_| "인터넷에 연결할 수 없음".to_string())?;
    let status = response.status().as_u16();
    if status != 200 {
        return Err(format!("사전을 받을 수 없음 (HTTP {status})"));
    }
    response
        .body_mut()
        .with_config()
        .limit(MAX_DOWNLOAD)
        .read_to_vec()
        .map_err(|_| "사전을 받다가 끊김".to_string())
}

/// Fetches and installs the dictionary. The screen calls this only after
/// the writer agreed to "맞춤법 사전을 받습니다 · 약 1MB · GPL 라이선스".
#[tauri::command]
pub async fn spell_download(app: AppHandle, state: State<'_, AppState>) -> Res<DictInfo> {
    let dir = spelling_dir(&app)?;
    let slot = state.speller.clone();
    blocking(move || {
        let bytes = fetch()?;
        let info = install::install(&dir, &bytes, install::SHA256).map_err(fail)?;
        *lock(&slot) = None;
        Ok(info)
    })
    .await
}

/// Takes the dictionary off this device.
#[tauri::command]
pub async fn spell_remove(app: AppHandle, state: State<'_, AppState>) -> Res<()> {
    let dir = spelling_dir(&app)?;
    *lock(&state.speller) = None;
    blocking(move || install::remove(&dir).map_err(fail)).await
}

type Slot = Arc<Mutex<Option<Arc<Speller>>>>;

fn lock(slot: &Slot) -> std::sync::MutexGuard<'_, Option<Arc<Speller>>> {
    slot.lock().unwrap_or_else(|p| p.into_inner())
}

/// The loaded dictionary, loading it the first time (about half a second).
fn speller(app: &AppHandle, slot: &Slot) -> Res<Arc<Speller>> {
    let mut loaded = lock(slot);
    if let Some(speller) = loaded.as_ref() {
        return Ok(speller.clone());
    }
    let dir = spelling_dir(app)?;
    if install::installed(&dir).is_none() {
        return Err(NO_DICTIONARY.into());
    }
    let speller = Arc::new(Speller::open(&install::folder(&dir)).map_err(fail)?);
    *loaded = Some(speller.clone());
    Ok(speller)
}

/// Words the dictionary does not know in each of `texts` (paragraphs), in
/// UTF-16 units. `known`: card names and aliases, the word lists and words
/// ignored for now.
#[tauri::command]
pub async fn spell_check(
    app: AppHandle,
    state: State<'_, AppState>,
    texts: Vec<String>,
    known: Vec<String>,
) -> Res<Vec<Vec<Issue>>> {
    let slot = state.speller.clone();
    blocking(move || {
        let speller = speller(&app, &slot)?;
        let known = Known::new(known);
        Ok(texts
            .iter()
            .map(|t| spell::check_text(&speller, &known, t))
            .collect())
    })
    .await
}

/// Words the writer may have meant.
#[tauri::command]
pub async fn spell_suggest(
    app: AppHandle,
    state: State<'_, AppState>,
    word: String,
) -> Res<Vec<String>> {
    let slot = state.speller.clone();
    blocking(move || Ok(speller(&app, &slot)?.suggest(&word))).await
}

/// 맞춤법 검사 over the whole work, or over `doc_ids`.
#[tauri::command]
pub async fn spell_check_work(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
    known: Vec<String>,
    doc_ids: Option<Vec<String>>,
) -> Res<WorkCheck> {
    let slot = state.speller.clone();
    blocking(move || {
        let speller = speller(&app, &slot)?;
        spell::check_work(Path::new(&root), &speller, &Known::new(known), doc_ids).map_err(fail)
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpellWords {
    /// 이 작품 사전 (in the project folder).
    project: Vec<String>,
    /// 이 기기 사전.
    device: Vec<String>,
}

fn word_lists(app: &AppHandle, root: &str) -> Res<SpellWords> {
    Ok(SpellWords {
        project: words::read(&words::project_words_file(Path::new(root))).map_err(fail)?,
        device: words::read(&spelling_words_file(app)?).map_err(fail)?,
    })
}

#[tauri::command]
pub async fn spell_words(app: AppHandle, root: String) -> Res<SpellWords> {
    word_lists(&app, &root)
}

/// Where a word goes: `project` (이 작품 사전) or `device` (이 기기 사전).
fn list_file(app: &AppHandle, root: &str, place: &str) -> Res<std::path::PathBuf> {
    match place {
        "device" => spelling_words_file(app),
        _ => Ok(words::project_words_file(Path::new(root))),
    }
}

/// 사전에 넣기.
#[tauri::command]
pub async fn spell_word_add(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
    word: String,
    place: String,
) -> Res<SpellWords> {
    {
        let _write = state.write();
        words::add(&list_file(&app, &root, &place)?, &word).map_err(fail)?;
    }
    word_lists(&app, &root)
}

/// 사전에서 빼기.
#[tauri::command]
pub async fn spell_word_remove(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
    word: String,
    place: String,
) -> Res<SpellWords> {
    {
        let _write = state.write();
        words::remove(&list_file(&app, &root, &place)?, &word).map_err(fail)?;
    }
    word_lists(&app, &root)
}
