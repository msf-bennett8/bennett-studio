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

/// Get a stable device identifier for vault key derivation.
/// Uses the app data directory path hash — stable per install.
#[command]
pub fn get_device_id() -> String {
    let app_dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join("com.bennett.studio");
    
    let path_str = app_dir.to_string_lossy();
    let hash = blake3::hash(path_str.as_bytes());
    hash.to_hex().to_string()
}
