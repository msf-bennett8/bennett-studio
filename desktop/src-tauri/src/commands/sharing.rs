use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Serialize, Deserialize, Debug)]
pub struct ShareRequest {
    pub database_id: String,
    pub expires_in_hours: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ShareInfo {
    pub id: String,
    pub url: String,
    pub expires_at: Option<String>,
}

// Phase 1: Updated share request matching new API
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateShareRequest {
    pub database_id: String,
    pub permission: Option<String>,
    pub tables: Option<Vec<String>>,
    pub duration_hours: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateShareResponse {
    pub code: String,
    pub url: String,
    pub token: String,
    pub expires_at: String,
    pub ice: Option<String>, // Base64 ICE candidates for P2P
}

#[command]
pub async fn create_share(req: CreateShareRequest) -> Result<CreateShareResponse, String> {
    // First create the share via engine API
    let client = reqwest::Client::new();
    let share_result = match client
        .post("http://localhost:3001/api/shares")
        .json(&req)
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => {
                        if let Some(data) = json.get("data") {
                            match serde_json::from_value::<CreateShareResponse>(data.clone()) {
                                Ok(share) => share,
                                Err(e) => return Err(format!("Parse error: {}", e)),
                            }
                        } else {
                            return Err("No data field".to_string());
                        }
                    }
                    Err(e) => return Err(format!("JSON error: {}", e)),
                }
            } else {
                return Err(format!("HTTP error: {}", resp.status()));
            }
        }
        Err(e) => return Err(format!("Request failed: {}", e)),
    };

    // Gather ICE candidates for P2P (fire and forget — don't fail if STUN fails)
    let ice_b64 = match gather_ice_candidates().await {
        Ok(ice) => {
            tracing::info!("ICE candidates gathered for share {}", share_result.code);
            Some(ice.to_base64())
        }
        Err(e) => {
            tracing::warn!("Failed to gather ICE candidates: {}", e);
            None
        }
    };

    Ok(CreateShareResponse {
        code: share_result.code,
        url: share_result.url,
        token: share_result.token,
        expires_at: share_result.expires_at,
        ice: ice_b64,
    })
}

/// Gather ICE candidates by calling relay binary
async fn gather_ice_candidates() -> Result<bennett_relay::transport::ice::IceCandidates, String> {
    // Run relay binary with --gather-ice flag
    let output = tokio::process::Command::new("bennett-relay")
        .arg("--gather-ice")
        .output()
        .await
        .map_err(|e| format!("Failed to run relay: {}", e))?;

    if !output.status.success() {
        return Err(format!("Relay failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let json = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&json)
        .map_err(|e| format!("Failed to parse ICE: {}", e))
}

#[command]
pub async fn revoke_share(code: String) -> Result<bool, String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "code": code,
        "reason": "host_revoked"
    });
    match client
        .delete(&format!("http://localhost:3001/api/shares/{}", code))
        .json(&body)
        .send()
        .await
    {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(e) => Err(format!("Request failed: {}", e)),
    }
}

#[command]
pub async fn list_shares() -> Result<serde_json::Value, String> {
    match reqwest::get("http://localhost:3001/api/shares").await {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => Ok(json),
                    Err(e) => Err(format!("JSON error: {}", e)),
                }
            } else {
                Err(format!("HTTP error: {}", resp.status()))
            }
        }
        Err(e) => Err(format!("Request failed: {}", e)),
    }
}
