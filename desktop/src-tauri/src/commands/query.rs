use serde::{Deserialize, Serialize};
use tauri::command;

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryRequest {
    pub sql: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct TableDataRequest {
    pub table: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub order_by: Option<String>,
    pub order_dir: Option<String>,
    pub filter: Option<String>,
}

#[command]
pub async fn execute_query(id: String, sql: String) -> Result<QueryResult, String> {
    let client = reqwest::Client::new();
    match client
        .post(&format!("http://localhost:3001/api/databases/{}/query", id))
        .json(&serde_json::json!({ "sql": sql }))
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => {
                        if let Some(data) = json.get("data") {
                            match serde_json::from_value(data.clone()) {
                                Ok(result) => Ok(result),
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
pub async fn get_schema(id: String) -> Result<Vec<serde_json::Value>, String> {
    match reqwest::get(&format!("http://localhost:3001/api/databases/{}/schema", id)).await {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => {
                        if let Some(data) = json.get("data") {
                            match serde_json::from_value(data.clone()) {
                                Ok(schema) => Ok(schema),
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
pub async fn get_table_data(id: String, req: TableDataRequest) -> Result<serde_json::Value, String> {
    let client = reqwest::Client::new();
    match client
        .post(&format!("http://localhost:3001/api/databases/{}/data", id))
        .json(&req)
        .send()
        .await
    {
        Ok(resp) => {
            if resp.status().is_success() {
                match resp.json::<serde_json::Value>().await {
                    Ok(json) => {
                        if let Some(data) = json.get("data") {
                            Ok(data.clone())
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
