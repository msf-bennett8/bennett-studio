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

/// Generate a stable device ID from OS-level paths.
/// Deterministic per machine+user, pure std — no external crates.
#[command]
pub fn get_device_id() -> Result<String, String> {
    // Get home dir from env (cross-platform)
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| "/tmp".to_string());

    let path = format!("{}/.local/share/bennett-studio", home);

    // FNV-1a 64-bit hash — pure std, no crates
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in path.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }

    Ok(format!("bennett-device-{:016x}", hash))
}
