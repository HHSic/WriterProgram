//! Files the app keeps in its settings folder.

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::error::Res;

fn config_file(app: &AppHandle, name: &str) -> Res<PathBuf> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join(name))
        .map_err(|e| e.to_string())
}

/// Projects opened lately, for the start screen.
pub fn recent_file(app: &AppHandle) -> Res<PathBuf> {
    config_file(app, "recent.json")
}

/// User manuscript format presets (내 서식), shared by all projects.
pub fn presets_file(app: &AppHandle) -> Res<PathBuf> {
    config_file(app, "presets.json")
}

/// Which drives are connected and which projects follow them.
pub fn registry_file(app: &AppHandle) -> Res<PathBuf> {
    config_file(app, "drives.json")
}

/// The app's registrations with the drives.
pub fn apps_file(app: &AppHandle) -> Res<PathBuf> {
    config_file(app, "drive-apps.json")
}

/// This device's creation journal settings: its device id and the on/off
/// switch (`writer_core::journal::Settings`).
pub fn journal_file(app: &AppHandle) -> Res<PathBuf> {
    config_file(app, "journal.json")
}

/// What each project looked like after its last pass (`sync/<id>.json`).
pub fn base_dir(app: &AppHandle) -> Res<PathBuf> {
    config_file(app, "sync")
}
