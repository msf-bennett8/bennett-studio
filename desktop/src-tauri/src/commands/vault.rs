//! Secure token vault — OS-native keychain integration
//! Uses macOS Keychain, Windows Credential Manager, Linux Secret Service

use serde::{Deserialize, Serialize};
use tauri::command;
use tracing::{info, warn, error};

const VAULT_SERVICE_NAME: &str = "bennett-studio-share-tokens";
const VAULT_USERNAME_PREFIX: &str = "bennett-share-";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultToken {
    pub code: String,
    pub token: String,
    pub db_id: String,
    pub db_name: String,
    pub created_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    pub code: String,
    pub db_id: String,
    pub db_name: String,
    pub created_at: String,
    pub expires_at: String,
}

/// Store a share token in the OS keychain
#[command]
pub async fn vault_store_token(entry: VaultToken) -> Result<bool, String> {
    let code = entry.code.clone(); // Save before shadowing
    let entry_name = format!("{}{}", VAULT_USERNAME_PREFIX, code);

    // Use keyring crate for cross-platform secure storage
    // keyring v3: Entry::new returns Result<Entry, Error>
    let keyring_entry = keyring::Entry::new(VAULT_SERVICE_NAME, &entry_name)
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;

    // Store encrypted token
    keyring_entry.set_password(&entry.token)
        .map_err(|e| format!("Failed to store token: {}", e))?;

    // Store metadata in a separate entry (non-sensitive, for listing)
    let meta_entry = keyring::Entry::new(VAULT_SERVICE_NAME, &format!("{}-meta", entry_name))
        .map_err(|e| format!("Failed to create meta entry: {}", e))?;

    let meta = serde_json::json!({
        "db_id": entry.db_id,
        "db_name": entry.db_name,
        "created_at": entry.created_at,
        "expires_at": entry.expires_at,
    });

    meta_entry.set_password(&meta.to_string())
        .map_err(|e| format!("Failed to store metadata: {}", e))?;

    info!("Stored token for share {} in OS keychain", code);
    Ok(true)
}

/// Retrieve a share token from the OS keychain
#[command]
pub async fn vault_get_token(code: String) -> Result<Option<String>, String> {
    let entry_name = format!("{}{}", VAULT_USERNAME_PREFIX, code);
    
    let entry = keyring::Entry::new(VAULT_SERVICE_NAME, &entry_name)
        .map_err(|e| format!("Failed to access keyring: {}", e))?;
    
    match entry.get_password() {
        Ok(token) => {
            info!("Retrieved token for share {} from OS keychain", code);
            Ok(Some(token))
        }
        Err(keyring::Error::NoEntry) => {
            warn!("No token found for share {} in keychain", code);
            Ok(None)
        }
        Err(e) => {
            error!("Failed to retrieve token: {}", e);
            Err(format!("Failed to retrieve token: {}", e))
        }
    }
}

/// List all stored share entries (metadata only, no tokens)
#[command]
pub async fn vault_list_entries() -> Result<Vec<VaultEntry>, String> {
    // keyring doesn't support listing, so we use a sidecar file for index
    // In production, use a proper keychain enumeration or maintain index
    // For now, return empty — frontend will use its own cache for listing
    Ok(vec![])
}

/// Remove a token from the vault
#[command]
pub async fn vault_remove_token(code: String) -> Result<bool, String> {
    let entry_name = format!("{}{}", VAULT_USERNAME_PREFIX, code);
    
    let entry = keyring::Entry::new(VAULT_SERVICE_NAME, &entry_name)
        .map_err(|e| format!("Failed to access keyring: {}", e))?;
    
    match entry.delete_credential() {
        Ok(_) => {
            // Also delete meta entry
            let meta_entry = keyring::Entry::new(VAULT_SERVICE_NAME, &format!("{}-meta", entry_name))
                .map_err(|e| format!("Failed to access meta entry: {}", e))?;
            let _ = meta_entry.delete_credential();
            
            info!("Removed token for share {} from OS keychain", code);
            Ok(true)
        }
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(e) => Err(format!("Failed to remove token: {}", e)),
    }
}

/// Check if vault is available
#[command]
pub async fn vault_status() -> Result<VaultStatus, String> {
    // Test keyring availability
    // keyring v3: Entry::new returns Result<Entry, Error>
    let test_entry = keyring::Entry::new(VAULT_SERVICE_NAME, "test-availability")
        .map_err(|e| format!("Keyring not available: {}", e))?;
    
    let available = test_entry.set_password("test").is_ok();

    if available {
        let _ = test_entry.delete_credential();
    }

    Ok(VaultStatus {
        available,
        r#type: "tauri_secure".to_string(),
        initialized: true,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStatus {
    pub available: bool,
    pub r#type: String,
    pub initialized: bool,
}
