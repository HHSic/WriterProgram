//! Creation journal (창작 일지): this device's on/off switch, what the editor
//! reports (writing sessions and pastes), and what the writer sees of it.
//! Saves, records and imports are journaled inside `writer_core` itself.

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, State};
use writer_core::journal::{self, EditorEvent, Report, Settings, Summary};

use crate::error::{Res, fail};
use crate::paths::journal_file;
use crate::state::AppState;

/// What the screen knows of the settings.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalSettings {
    /// This device's id, to tell its journal from other devices' in a check.
    device: String,
    enabled: bool,
    noticed: bool,
    /// Time stamps allowed; null until the writer has been asked.
    anchor: Option<bool>,
    /// Ask before each stamp instead of taking it.
    anchor_ask: bool,
    /// Say so when a stamp came.
    anchor_notify: bool,
}

impl From<&Settings> for JournalSettings {
    fn from(s: &Settings) -> Self {
        JournalSettings {
            device: s.device.clone(),
            enabled: s.enabled,
            noticed: s.noticed,
            anchor: s.anchor,
            anchor_ask: s.anchor_ask,
            anchor_notify: s.anchor_notify,
        }
    }
}

/// Reads this device's settings (making its id the first time) and turns the
/// journal on or off to match. Called once when the app starts; a journal
/// that cannot be set up stays off. Also starts the minute check that writes
/// gathered saves once they are ten minutes old (`journal::flush_due`).
pub fn init(app: &AppHandle) {
    let settings = journal_file(app).and_then(|path| journal::load_settings(&path).map_err(fail));
    match settings {
        Ok(settings) => journal::set_device(settings.active_device()),
        Err(e) => eprintln!("창작 일지를 켜지 못함: {e}"),
    }
    std::thread::spawn(|| {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
            journal::flush_due(chrono::Utc::now());
        }
    });
}

/// Writes the saves still gathering: for `root` when the writer leaves the
/// project, for every project (none given) when the window closes. The app
/// also does the latter when it exits (`lib.rs`).
#[tauri::command]
pub async fn journal_flush(root: Option<String>) -> Res<()> {
    match root {
        Some(root) => journal::flush(Path::new(&root)),
        None => journal::flush_all(),
    }
    Ok(())
}

#[tauri::command]
pub async fn journal_settings(app: AppHandle) -> Res<JournalSettings> {
    let settings = journal::load_settings(&journal_file(&app)?).map_err(fail)?;
    Ok(JournalSettings::from(&settings))
}

/// Turns the journal on or off for this device, notes that the writer has
/// seen the first-use notice, or allows (or not) the daily time stamps.
#[tauri::command]
pub async fn journal_set(
    app: AppHandle,
    state: State<'_, AppState>,
    enabled: Option<bool>,
    noticed: Option<bool>,
    anchor: Option<bool>,
    anchor_ask: Option<bool>,
    anchor_notify: Option<bool>,
) -> Res<JournalSettings> {
    let _write = state.write();
    let path = journal_file(&app)?;
    let mut settings = journal::load_settings(&path).map_err(fail)?;
    if let Some(enabled) = enabled {
        settings.enabled = enabled;
    }
    if let Some(noticed) = noticed {
        settings.noticed = noticed;
    }
    if anchor.is_some() {
        settings.anchor = anchor;
    }
    if let Some(ask) = anchor_ask {
        settings.anchor_ask = ask;
    }
    if let Some(notify) = anchor_notify {
        settings.anchor_notify = notify;
    }
    journal::save_settings(&path, &settings).map_err(fail)?;
    journal::set_device(settings.active_device());
    Ok(JournalSettings::from(&settings))
}

/// A writing session or a paste from the editor; nothing while the journal
/// is off.
#[tauri::command]
pub async fn journal_event(
    state: State<'_, AppState>,
    root: String,
    event: EditorEvent,
) -> Res<()> {
    let Some(device) = journal::device() else {
        return Ok(());
    };
    let Some(entry) = journal::editor_entry(event).map_err(fail)? else {
        return Ok(());
    };
    let _write = state.write();
    journal::append(Path::new(&root), &device, &entry).map_err(fail)
}

#[tauri::command]
pub async fn journal_summary(app: AppHandle, root: String) -> Res<Summary> {
    let settings = journal::load_settings(&journal_file(&app)?).map_err(fail)?;
    // Saves still gathering count too.
    journal::flush(Path::new(&root));
    journal::summary(Path::new(&root), Some(&settings.device)).map_err(fail)
}

#[tauri::command]
pub async fn journal_verify(root: String) -> Res<Report> {
    journal::flush(Path::new(&root));
    journal::verify(Path::new(&root)).map_err(fail)
}
