use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DatabaseInstance {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub db_type: String,
    pub version: String,
    pub status: String,
    pub port: u16,
    pub size: String,
    pub created_at: String,
    pub container_id: Option<String>,
    pub volume_name: Option<String>,
    pub env_vars: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateDatabaseRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub db_type: String,
    pub version: String,
}

#[command]
pub async fn list_databases() -> Result<Vec<DatabaseInstance>, String> {
    match reqwest::get("http://localhost:3001/api/databases").await {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => {
                        if let Some(data) = json.get("data") {
                            match serde_json::from_value(data.clone()) {
                                Ok(databases) => Ok(databases),
                                Err(e) => Err(format!("Parse error: {}", e)),
                            }
                        } else {
                            Err("No data field in response".to_string())
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
pub async fn create_database(req: CreateDatabaseRequest) -> Result<DatabaseInstance, String> {
    let client = reqwest::Client::new();
    match client
        .post("http://localhost:3001/api/databases")
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
                                Ok(db) => Ok(db),
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
pub async fn delete_database(id: String) -> Result<bool, String> {
    let client = reqwest::Client::new();
    match client
        .delete(&format!("http://localhost:3001/api/databases/{}", id))
        .send()
        .await
    {
        Ok(resp) => Ok(resp.status().is_success()),
        Err(e) => Err(format!("Request failed: {}", e)),
    }
}

#[command]
pub async fn start_database(id: String) -> Result<DatabaseInstance, String> {
    let client = reqwest::Client::new();
    match client
        .post(&format!("http://localhost:3001/api/databases/{}/start", id))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => {
                        if let Some(data) = json.get("data") {
                            match serde_json::from_value(data.clone()) {
                                Ok(db) => Ok(db),
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
pub async fn stop_database(id: String) -> Result<DatabaseInstance, String> {
    let client = reqwest::Client::new();
    match client
        .post(&format!("http://localhost:3001/api/databases/{}/stop", id))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => {
                        if let Some(data) = json.get("data") {
                            match serde_json::from_value(data.clone()) {
                                Ok(db) => Ok(db),
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
