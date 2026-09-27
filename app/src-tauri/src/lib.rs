mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(commands::AppState::default())
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
            commands::reveal,
        ])
        .run(tauri::generate_context!())
        .expect("error while running WriterProgram");
}
