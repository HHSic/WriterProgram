mod commands;
mod drives;
mod watch;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = commands::AppState::default();
    let own = state.own.clone();
    writer_core::store::on_write(move |path, bytes| own.record(path, bytes));
    drives::init_secrets();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(state)
        .manage(drives::DriveState::default())
        .invoke_handler(tauri::generate_handler![
            commands::recent_list,
            commands::recent_remove,
            commands::default_location,
            commands::project_create,
            commands::project_open,
            commands::project_overview,
            commands::project_update,
            commands::part_add,
            commands::part_rename,
            commands::part_remove,
            commands::doc_add,
            commands::doc_move,
            commands::doc_trash,
            commands::doc_load,
            commands::doc_save,
            commands::doc_update_meta,
            commands::snapshot_list,
            commands::snapshot_create,
            commands::snapshot_load,
            commands::snapshot_restore,
            commands::trash_list,
            commands::trash_restore,
            commands::trash_delete,
            commands::export_text,
            commands::export_txt,
            commands::export_file,
            commands::import_preview,
            commands::import_commit,
            commands::format_catalog,
            commands::format_save_preset,
            commands::format_delete_preset,
            commands::format_estimate,
            commands::search,
            commands::replace_all,
            commands::card_load,
            commands::card_create,
            commands::card_save,
            commands::card_trash,
            commands::card_appearances,
            commands::card_counts,
            commands::card_type_add,
            commands::card_type_update,
            commands::card_type_remove,
            commands::note_list,
            commands::note_create,
            commands::note_save,
            commands::note_trash,
            commands::doc_keep,
            commands::copy_load,
            commands::copy_resolve,
            commands::storage_places,
            commands::storage_of,
            commands::project_move,
            commands::project_watch,
            commands::project_unwatch,
            drives::drive_status,
            drives::drive_connect,
            drives::drive_cancel,
            drives::drive_disconnect,
            drives::drive_projects,
            drives::drive_fetch,
            drives::project_link_get,
            drives::project_link,
            drives::project_unlink,
            drives::project_sync,
            commands::reveal,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WriterProgram");
}
