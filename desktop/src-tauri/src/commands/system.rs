use serde::Serialize;
use tauri::command;

#[derive(Serialize, Debug)]
pub struct SystemInfo {
    pub os: String,
    pub arch: String,
    pub version: String,
}

#[command]
pub fn get_system_info() -> Result<SystemInfo, String> {
    Ok(SystemInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
