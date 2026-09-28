//! Commands the screen calls. Each one is a thin wrapper over `writer_core`;
//! errors come back as a short reason in screen words.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};

use chrono::Duration;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_opener::OpenerExt;
use writer_core::cards::{self, Appearance, Card, CardSummary, CardType};
use writer_core::copies::{self, Resolve};
use writer_core::count::{Counts, count_blocks};
use writer_core::doc::{self, DocFile, DocMeta, MetaPatch, SaveGuard, SaveOutcome, Section};
use writer_core::export::{self, DocOptions, ExportItem, FileKind, TextOptions};
use writer_core::format::{self, Catalog, ManuscriptFormat, UserPreset};
use writer_core::markup::Body;
use writer_core::notes::{self, NewNote, Note};
use writer_core::places::{self, Env, Place};
use writer_core::project::{self, NewDoc, NewProject, Overview, ProjectInfo, ProjectPatch};
use writer_core::recent::{self, RecentItem};
use writer_core::search::{self, ReplaceOutcome, SearchQuery, SearchResult};
use writer_core::snapshot::{self, SnapshotInfo};
use writer_core::trash::{self, TrashItem};

use crate::watch::{self, OwnWrites, Watching};

/// How often an automatic record is kept while writing.
const AUTO_RECORD_EVERY_MINUTES: i64 = 10;

#[derive(Default)]
pub struct AppState {
    /// Serializes writes so two saves never read-modify-write the same file at once.
    lock: Mutex<()>,
    /// What the app itself wrote, to tell it from other devices' changes.
    pub own: Arc<OwnWrites>,
    /// The open project's folder watcher.
    watching: Mutex<Option<Watching>>,
}

impl AppState {
    pub fn write(&self) -> MutexGuard<'_, ()> {
        self.lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The lock itself, for work that takes it around each write (sync passes).
    pub fn lock(&self) -> &Mutex<()> {
        &self.lock
    }

    fn watch(&self, watching: Option<Watching>) {
        let mut slot = self
            .watching
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *slot = watching;
    }

    /// Stops watching `root`, unless another project is watched by now.
    fn unwatch(&self, root: &str) {
        let mut slot = self
            .watching
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if slot.as_ref().is_some_and(|w| w.root == root) {
            *slot = None;
        }
    }
}

type Res<T> = Result<T, String>;

fn fail(e: writer_core::Error) -> String {
    e.user_message()
}

fn config_file(app: &AppHandle, name: &str) -> Res<PathBuf> {
    app.path()
        .app_config_dir()
        .map(|dir| dir.join(name))
        .map_err(|e| e.to_string())
}

pub(crate) fn recent_file(app: &AppHandle) -> Res<PathBuf> {
    config_file(app, "recent.json")
}

/// User manuscript format presets (내 서식), shared by all projects.
fn presets_file(app: &AppHandle) -> Res<PathBuf> {
    config_file(app, "presets.json")
}

/// A document with its text for the editor.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocData {
    meta: DocMeta,
    body: Body,
    counts: Counts,
    /// Fingerprint of the text, sent back with saves (see `SaveGuard`).
    rev: String,
}

impl From<DocFile> for DocData {
    fn from(file: DocFile) -> Self {
        let counts = count_blocks(&file.body);
        let rev = file.rev();
        DocData {
            meta: file.meta,
            body: Body::new(file.body),
            counts,
            rev,
        }
    }
}

#[tauri::command]
pub async fn recent_list(app: AppHandle) -> Res<Vec<RecentItem>> {
    Ok(recent::list(&recent_file(&app)?))
}

#[tauri::command]
pub async fn recent_remove(app: AppHandle, path: String) -> Res<()> {
    recent::remove(&recent_file(&app)?, &path).map_err(fail)
}

/// Suggested place for new projects: Documents/WriterProgram.
#[tauri::command]
pub async fn default_location(app: AppHandle) -> Res<String> {
    let docs = app.path().document_dir().map_err(|e| e.to_string())?;
    Ok(docs.join("WriterProgram").to_string_lossy().into_owned())
}

#[tauri::command]
pub async fn project_create(
    app: AppHandle,
    state: State<'_, AppState>,
    opts: NewProject,
) -> Res<Overview> {
    let _write = state.write();
    let root = project::create(&opts).map_err(fail)?;
    let overview = project::open(&root).map_err(fail)?;
    let _ = recent::touch(&recent_file(&app)?, &overview);
    Ok(overview)
}

#[tauri::command]
pub async fn project_open(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Res<Overview> {
    let _write = state.write();
    let overview = project::open(Path::new(&path)).map_err(fail)?;
    let _ = recent::touch(&recent_file(&app)?, &overview);
    Ok(overview)
}

#[tauri::command]
pub async fn project_overview(root: String) -> Res<Overview> {
    project::overview(Path::new(&root)).map_err(fail)
}

#[tauri::command]
pub async fn project_update(
    state: State<'_, AppState>,
    root: String,
    patch: ProjectPatch,
) -> Res<ProjectInfo> {
    let _write = state.write();
    project::update(Path::new(&root), &patch).map_err(fail)
}

#[tauri::command]
pub async fn part_add(state: State<'_, AppState>, root: String, title: String) -> Res<String> {
    let _write = state.write();
    project::add_part(Path::new(&root), &title).map_err(fail)
}

#[tauri::command]
pub async fn part_rename(
    state: State<'_, AppState>,
    root: String,
    part_id: String,
    title: String,
) -> Res<()> {
    let _write = state.write();
    project::rename_part(Path::new(&root), &part_id, &title).map_err(fail)
}

#[tauri::command]
pub async fn part_remove(state: State<'_, AppState>, root: String, part_id: String) -> Res<()> {
    let _write = state.write();
    project::remove_part(Path::new(&root), &part_id).map_err(fail)
}

#[tauri::command]
pub async fn doc_add(state: State<'_, AppState>, root: String, spec: NewDoc) -> Res<String> {
    let _write = state.write();
    project::add_doc(Path::new(&root), &spec).map_err(fail)
}

#[tauri::command]
pub async fn doc_move(
    state: State<'_, AppState>,
    root: String,
    doc_id: String,
    part_id: Option<String>,
    index: usize,
) -> Res<()> {
    let _write = state.write();
    project::move_doc(Path::new(&root), &doc_id, part_id.as_deref(), index).map_err(fail)
}

#[tauri::command]
pub async fn doc_trash(state: State<'_, AppState>, root: String, doc_id: String) -> Res<TrashItem> {
    let _write = state.write();
    trash::trash_doc(Path::new(&root), &doc_id).map_err(fail)
}

#[tauri::command]
pub async fn doc_load(root: String, doc_id: String) -> Res<DocData> {
    doc::load(Path::new(&root), &doc_id)
        .map(DocData::from)
        .map_err(fail)
}

/// Saves the editor's text. `base` is the fingerprint of the text the editor
/// started from: when the text on disk changed since (another device), the
/// save is refused unless `force` (see `doc::save_body`).
#[tauri::command]
pub async fn doc_save(
    state: State<'_, AppState>,
    root: String,
    doc_id: String,
    body: Body,
    base: Option<String>,
    force: Option<bool>,
) -> Res<SaveOutcome> {
    let _write = state.write();
    doc::save_body(
        Path::new(&root),
        &doc_id,
        body.content,
        Duration::minutes(AUTO_RECORD_EVERY_MINUTES),
        SaveGuard {
            base: base.as_deref(),
            force: force.unwrap_or(false),
        },
    )
    .map_err(fail)
}

/// Keeps the editor's text as a record, before another device's text is
/// loaded in its place.
#[tauri::command]
pub async fn doc_keep(
    state: State<'_, AppState>,
    root: String,
    doc_id: String,
    body: Body,
    kind: String,
) -> Res<Option<SnapshotInfo>> {
    let _write = state.write();
    doc::keep_record(Path::new(&root), &doc_id, body.content, &kind).map_err(fail)
}

#[tauri::command]
pub async fn doc_update_meta(
    state: State<'_, AppState>,
    root: String,
    doc_id: String,
    patch: MetaPatch,
) -> Res<DocMeta> {
    let _write = state.write();
    doc::update_meta(Path::new(&root), &doc_id, &patch).map_err(fail)
}

#[tauri::command]
pub async fn snapshot_list(root: String, doc_id: String) -> Res<Vec<SnapshotInfo>> {
    snapshot::list(Path::new(&root), &doc_id).map_err(fail)
}

/// "지금 원고 보관": keeps a record by hand.
#[tauri::command]
pub async fn snapshot_create(
    state: State<'_, AppState>,
    root: String,
    doc_id: String,
    name: String,
) -> Res<SnapshotInfo> {
    let _write = state.write();
    let root = Path::new(&root);
    let current = doc::load(root, &doc_id).map_err(fail)?;
    snapshot::create(root, &current, "manual", &name).map_err(fail)
}

#[tauri::command]
pub async fn snapshot_load(root: String, doc_id: String, snapshot_id: String) -> Res<DocData> {
    snapshot::load(Path::new(&root), &doc_id, &snapshot_id)
        .map(DocData::from)
        .map_err(fail)
}

/// "이 때로 되돌리기".
#[tauri::command]
pub async fn snapshot_restore(
    state: State<'_, AppState>,
    root: String,
    doc_id: String,
    snapshot_id: String,
) -> Res<SnapshotInfo> {
    let _write = state.write();
    snapshot::restore(Path::new(&root), &doc_id, &snapshot_id).map_err(fail)
}

#[tauri::command]
pub async fn trash_list(root: String) -> Res<Vec<TrashItem>> {
    trash::list(Path::new(&root)).map_err(fail)
}

#[tauri::command]
pub async fn trash_restore(state: State<'_, AppState>, root: String, trash_id: String) -> Res<()> {
    let _write = state.write();
    trash::restore(Path::new(&root), &trash_id)
        .map(|_| ())
        .map_err(fail)
}

#[tauri::command]
pub async fn trash_delete(state: State<'_, AppState>, root: String, trash_id: String) -> Res<()> {
    let _write = state.write();
    trash::delete(Path::new(&root), &trash_id).map_err(fail)
}

/// Text for the clipboard.
#[tauri::command]
pub async fn export_text(root: String, items: Vec<ExportItem>, opts: TextOptions) -> Res<String> {
    export::items_text(Path::new(&root), &items, &opts).map_err(fail)
}

#[tauri::command]
pub async fn export_txt(
    root: String,
    items: Vec<ExportItem>,
    opts: TextOptions,
    dest: String,
    per_doc: bool,
) -> Res<Vec<String>> {
    export::export_txt(Path::new(&root), &items, &opts, Path::new(&dest), per_doc)
        .map(|files| {
            files
                .into_iter()
                .map(|f| f.to_string_lossy().into_owned())
                .collect()
        })
        .map_err(fail)
}

/// Shows a file or folder in the system file manager.
#[tauri::command]
pub async fn reveal(app: AppHandle, path: String) -> Res<()> {
    app.opener()
        .reveal_item_in_dir(&path)
        .map_err(|e| e.to_string())
}

/// Built-in and user manuscript formats, fonts and paper sizes.
#[tauri::command]
pub async fn format_catalog(app: AppHandle) -> Res<Catalog> {
    Ok(format::catalog(&presets_file(&app)?))
}

/// "내 서식으로 저장".
#[tauri::command]
pub async fn format_save_preset(
    app: AppHandle,
    name: String,
    format: ManuscriptFormat,
) -> Res<Vec<UserPreset>> {
    format::save_user_preset(&presets_file(&app)?, &name, &format).map_err(fail)
}

#[tauri::command]
pub async fn format_delete_preset(app: AppHandle, name: String) -> Res<Vec<UserPreset>> {
    format::delete_user_preset(&presets_file(&app)?, &name).map_err(fail)
}

/// Estimated pages of the whole manuscript in a format being tried.
#[tauri::command]
pub async fn format_estimate(root: String, format: ManuscriptFormat) -> Res<Option<u32>> {
    project::estimate_pages(Path::new(&root), &format).map_err(fail)
}

/// Word (docx) or 한글 (HWPX) export with a manuscript format.
#[tauri::command]
pub async fn export_file(
    root: String,
    items: Vec<ExportItem>,
    opts: DocOptions,
    format: ManuscriptFormat,
    kind: FileKind,
    dest: String,
    per_doc: bool,
) -> Res<Vec<String>> {
    export::export_file(
        Path::new(&root),
        &items,
        &opts,
        &format,
        kind,
        Path::new(&dest),
        per_doc,
    )
    .map(|files| {
        files
            .into_iter()
            .map(|f| f.to_string_lossy().into_owned())
            .collect()
    })
    .map_err(fail)
}

/// 작품 전체 찾기.
#[tauri::command]
pub async fn search(root: String, query: SearchQuery) -> Res<SearchResult> {
    search::search(Path::new(&root), &query).map_err(fail)
}

/// 모두 바꾸기. Each changed document first gets a "바꾸기 전" record.
#[tauri::command]
pub async fn replace_all(
    state: State<'_, AppState>,
    root: String,
    query: SearchQuery,
    replacement: String,
) -> Res<ReplaceOutcome> {
    let _write = state.write();
    search::replace_all(Path::new(&root), &query, &replacement).map_err(fail)
}

#[tauri::command]
pub async fn card_load(root: String, card_id: String) -> Res<Card> {
    cards::load(Path::new(&root), &card_id).map_err(fail)
}

#[tauri::command]
pub async fn card_create(
    state: State<'_, AppState>,
    root: String,
    type_id: String,
    name: String,
) -> Res<Card> {
    let _write = state.write();
    cards::create(Path::new(&root), &type_id, &name).map_err(fail)
}

#[tauri::command]
pub async fn card_save(state: State<'_, AppState>, root: String, card: Card) -> Res<CardSummary> {
    let _write = state.write();
    cards::save(Path::new(&root), &card).map_err(fail)
}

#[tauri::command]
pub async fn card_trash(
    state: State<'_, AppState>,
    root: String,
    card_id: String,
) -> Res<TrashItem> {
    let _write = state.write();
    trash::trash_card(Path::new(&root), &card_id).map_err(fail)
}

/// Chapters where a card's names appear.
#[tauri::command]
pub async fn card_appearances(root: String, card_id: String) -> Res<Vec<Appearance>> {
    cards::appearances(Path::new(&root), &card_id).map_err(fail)
}

/// For each card, in how many chapters it appears.
#[tauri::command]
pub async fn card_counts(root: String) -> Res<Vec<(String, usize)>> {
    cards::appearance_counts(Path::new(&root)).map_err(fail)
}

#[tauri::command]
pub async fn card_type_add(
    state: State<'_, AppState>,
    root: String,
    name: String,
) -> Res<CardType> {
    let _write = state.write();
    cards::add_type(Path::new(&root), &name).map_err(fail)
}

#[tauri::command]
pub async fn card_type_update(state: State<'_, AppState>, root: String, kind: CardType) -> Res<()> {
    let _write = state.write();
    cards::update_type(Path::new(&root), &kind).map_err(fail)
}

#[tauri::command]
pub async fn card_type_remove(
    state: State<'_, AppState>,
    root: String,
    type_id: String,
) -> Res<()> {
    let _write = state.write();
    cards::remove_type(Path::new(&root), &type_id).map_err(fail)
}

// ---------------------------------------------------------------------------
// Notes (메모)

#[tauri::command]
pub async fn note_list(root: String) -> Res<Vec<Note>> {
    notes::list(Path::new(&root)).map_err(fail)
}

#[tauri::command]
pub async fn note_create(state: State<'_, AppState>, root: String, spec: NewNote) -> Res<Note> {
    let _write = state.write();
    notes::create(Path::new(&root), &spec).map_err(fail)
}

#[tauri::command]
pub async fn note_save(state: State<'_, AppState>, root: String, note: Note) -> Res<Note> {
    let _write = state.write();
    notes::save(Path::new(&root), &note).map_err(fail)
}

#[tauri::command]
pub async fn note_trash(
    state: State<'_, AppState>,
    root: String,
    note_id: String,
) -> Res<TrashItem> {
    let _write = state.write();
    trash::trash_note(Path::new(&root), &note_id).map_err(fail)
}

// ---------------------------------------------------------------------------
// Other devices: copies left by sync programs, where projects are kept, and
// watching the open project for changes that arrive.

/// A copy of a chapter or planning document, to compare with the original.
#[tauri::command]
pub async fn copy_load(root: String, section: Section, file: String) -> Res<DocData> {
    copies::load(Path::new(&root), section, &file)
        .map(DocData::from)
        .map_err(fail)
}

#[tauri::command]
pub async fn copy_resolve(
    state: State<'_, AppState>,
    root: String,
    section: Section,
    file: String,
    action: Resolve,
) -> Res<()> {
    let _write = state.write();
    copies::resolve(Path::new(&root), section, &file, action).map_err(fail)
}

fn detect_places(app: &AppHandle) -> Vec<Place> {
    places::detect(&Env::current(app.path().document_dir().ok()))
}

/// Folders that OneDrive, Google Drive, Dropbox or iCloud keep in step with
/// other devices, then this computer only.
#[tauri::command]
pub async fn storage_places(app: AppHandle) -> Res<Vec<Place>> {
    Ok(detect_places(&app))
}

/// The sync folder a path is in, if any.
#[tauri::command]
pub async fn storage_of(app: AppHandle, path: String) -> Res<Option<Place>> {
    let places = detect_places(&app);
    Ok(places::place_of(Path::new(&path), &places).cloned())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveOutcome {
    overview: Overview,
    left_behind: bool,
}

/// Moves the project folder into `dest` and opens it there.
#[tauri::command]
pub async fn project_move(
    app: AppHandle,
    state: State<'_, AppState>,
    root: String,
    dest: String,
) -> Res<MoveOutcome> {
    // A watched folder cannot be renamed on Windows.
    state.watch(None);
    let _write = state.write();
    let moved = project::relocate(Path::new(&root), Path::new(&dest)).map_err(fail)?;
    let overview = project::open(Path::new(&moved.root)).map_err(fail)?;
    let recent = recent_file(&app)?;
    let _ = recent::remove(&recent, &root);
    let _ = recent::touch(&recent, &overview);
    Ok(MoveOutcome {
        overview,
        left_behind: moved.left_behind,
    })
}

/// Starts telling the screen about changes in the project folder that the
/// app did not make (event "project-changed").
#[tauri::command]
pub async fn project_watch(app: AppHandle, state: State<'_, AppState>, root: String) -> Res<()> {
    state.watch(None);
    state.watch(Some(watch::start(&app, &root)?));
    Ok(())
}

#[tauri::command]
pub async fn project_unwatch(state: State<'_, AppState>, root: String) -> Res<()> {
    state.unwatch(&root);
    Ok(())
}
