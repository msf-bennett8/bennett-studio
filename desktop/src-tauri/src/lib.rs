// lib.rs — Tauri desktop app entry point
// Exposes commands to the frontend via invoke()

pub mod commands;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::database::list_databases,
            commands::database::create_database,
            commands::database::delete_database,
            commands::database::start_database,
            commands::database::stop_database,
            commands::query::execute_query,
            commands::query::get_schema,
            commands::sharing::create_share,
            commands::sharing::revoke_share,
            commands::sharing::list_shares,
            commands::vault::vault_store_token,
            commands::vault::vault_get_token,
            commands::vault::vault_list_entries,
            commands::vault::vault_remove_token,
            commands::vault::vault_status,
            commands::system::get_system_info,
            commands::system::get_device_id,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
