// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::time::Duration;
use tauri::Manager;
//to start the window use use tauri::{Manager, Emitter};
mod commands;

#[tauri::command]
fn get_engine_status() -> Result<String, String> {
    match reqwest::blocking::get("http://localhost:3001/api/health") {
        Ok(resp) => {
            if resp.status().is_success() {
                Ok("running".to_string())
            } else {
                Ok("unhealthy".to_string())
            }
        }
        Err(_) => Ok("stopped".to_string()),
    }
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_deep_link::init())
        .invoke_handler(tauri::generate_handler![
            get_engine_status,
            commands::database::list_databases,
            commands::database::create_database,
            commands::database::delete_database,
            commands::database::start_database,
            commands::database::stop_database,
            commands::query::execute_query,
            commands::query::get_schema,
            commands::query::get_table_data,
            commands::sharing::create_share,
            commands::sharing::revoke_share,
            commands::system::get_system_info,
        ])
        .setup(|app| {
            let app_handle = app.handle().clone();

            // Show window immediately — don't block on engine
            if let Some(window) = app_handle.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }

            // Register deep link handler for bennett://share/CODE?t=JWT
            let deep_link_handle = app_handle.clone();
            app_handle.deep_link().on_open_url(move |event| {
                let url = event.url();
                tracing::info!("Deep link received: {}", url);
                
                // Parse bennett://share/CODE?t=JWT
                if let Some(path) = url.strip_prefix("bennett://share/") {
                    let parts: Vec<&str> = path.split('?').collect();
                    if parts.len() >= 2 {
                        let code = parts[0];
                        let query = parts[1];
                        if let Some(token) = query.strip_prefix("t=") {
                            // Show window if hidden
                            if let Some(window) = deep_link_handle.get_webview_window("main") {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                            
                            // Emit event to frontend with full share URL
                            let share_url = format!("https://share.bennett.studio/db/{}?t={}", code, token);
                            let _ = deep_link_handle.emit("deep-link-share", serde_json::json!({
                                "code": code,
                                "token": token,
                                "shareUrl": share_url,
                                "source": "deep-link"
                            }));
                            
                            tracing::info!("Deep link processed: code={}, token_len={}", code, token.len());
                        } else {
                            tracing::warn!("Deep link missing token parameter: {}", url);
                        }
                    } else {
                        tracing::warn!("Deep link missing query parameters: {}", url);
                    }
                } else {
                    tracing::warn!("Unknown deep link format: {}", url);
                }
            });

            // Background engine health check
            let health_handle = app_handle.clone();
            std::thread::spawn(move || {
                let mut ready = false;
                for _ in 0..30 {
                    std::thread::sleep(Duration::from_secs(1));
                    if let Ok(resp) = reqwest::blocking::get("http://localhost:3001/api/health") {
                        if resp.status().is_success() {
                            ready = true;
                            break;
                        }
                    }
                }
                let _ = health_handle.emit("engine-status", if ready { "ready" } else { "timeout" });
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
