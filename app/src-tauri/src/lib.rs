mod browser;
mod commands;
mod drives;
mod error;
mod paths;
mod state;
mod watch;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = state::AppState::default();
    let own = state.own.clone();
    writer_core::store::on_write(move |path, bytes| own.record(path, bytes));
    drives::keyring::init_secrets();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .manage(drives::connect::DriveState::default())
        .setup(|app| {
            commands::journal::init(app.handle());
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ai::ai_settings,
            commands::ai::ai_settings_set,
            commands::ai::ai_key_set,
            commands::ai::ai_key_remove,
            commands::ai::ai_connection_check,
            commands::ai::ai_preview,
            commands::ai::ai_summarize,
            commands::ai::ai_check,
            commands::project::recent_list,
            commands::project::recent_remove,
            commands::project::default_location,
            commands::project::project_create,
            commands::project::project_open,
            commands::project::project_overview,
            commands::project::project_update,
            commands::project::part_add,
            commands::project::part_rename,
            commands::project::part_remove,
            commands::project::project_move,
            commands::project::project_size,
            commands::project::reveal,
            commands::doc::doc_add,
            commands::doc::doc_move,
            commands::doc::doc_trash,
            commands::doc::doc_load,
            commands::doc::doc_save,
            commands::doc::doc_keep,
            commands::doc::doc_update_meta,
            commands::doc::snapshot_list,
            commands::doc::snapshot_create,
            commands::doc::snapshot_load,
            commands::doc::snapshot_restore,
            commands::doc::records_tidy,
            commands::doc::trash_list,
            commands::doc::trash_restore,
            commands::doc::trash_delete,
            commands::output::export_text,
            commands::output::export_txt,
            commands::output::export_file,
            commands::output::import_preview,
            commands::output::import_commit,
            commands::output::format_catalog,
            commands::output::format_save_preset,
            commands::output::format_delete_preset,
            commands::output::format_estimate,
            commands::exchange::exchange_send,
            commands::exchange::exchange_list,
            commands::exchange::exchange_read,
            commands::exchange::exchange_review,
            commands::exchange::exchange_apply,
            commands::search::search,
            commands::search::replace_all,
            commands::cards::card_load,
            commands::cards::card_create,
            commands::cards::card_save,
            commands::cards::card_trash,
            commands::cards::card_appearances,
            commands::cards::card_counts,
            commands::cards::card_type_add,
            commands::cards::card_type_update,
            commands::cards::card_type_remove,
            commands::notes::note_list,
            commands::notes::note_create,
            commands::notes::note_save,
            commands::notes::note_trash,
            commands::journal::journal_settings,
            commands::journal::journal_set,
            commands::journal::journal_event,
            commands::journal::journal_summary,
            commands::journal::journal_verify,
            commands::anchor::journal_anchor,
            commands::proof::proof_preview,
            commands::proof::proof_make,
            commands::sync_folder::copy_load,
            commands::sync_folder::copy_resolve,
            commands::sync_folder::storage_places,
            commands::sync_folder::storage_of,
            commands::sync_folder::project_watch,
            commands::sync_folder::project_unwatch,
            browser::browser_open,
            browser::browser_bounds,
            browser::browser_navigate,
            browser::browser_step,
            browser::browser_close,
            browser::browser_clip,
            drives::connect::drive_status,
            drives::connect::drive_connect,
            drives::connect::drive_cancel,
            drives::connect::drive_disconnect,
            drives::projects::drive_projects,
            drives::projects::drive_fetch,
            drives::projects::project_link_get,
            drives::projects::project_link,
            drives::projects::project_unlink,
            drives::projects::project_sync,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WriterProgram");
}
