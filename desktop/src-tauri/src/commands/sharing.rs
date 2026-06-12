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

#[command]
pub async fn create_share(req: ShareRequest) -> Result<ShareInfo, String> {
    let client = reqwest::Client::new();
    match client
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
                            match serde_json::from_value(data.clone()) {
                                Ok(share) => Ok(share),
                                Err(e) => Err(format!("Parse error: {}", e)),
                            }
                        } else {
                            Err("No data field".to_string())
                        }
                    }
                    Err(e) => Err(format!("JSON error: {}", e)),
                }
            } else {
                Err(format!("HTTP error: {}", resp.status()))
            }
        }
        Err(e) => Err(format!("Request failed: {}", e)),
    }
}

#[command]
pub async fn revoke_share(id: String) -> Result<bool, String> {
    let client = reqwest::Client::new();
    match client
        .delete(&format!("http://localhost:3001/api/shares/{}", id))
        .send()
        .await
    {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(e) => Err(format!("Request failed: {}", e)),
    }
}
