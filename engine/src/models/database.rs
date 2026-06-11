use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DatabaseStatus {
    Running,
    Stopped,
    Starting,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseInstance {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub db_type: String,
    pub version: String,
    pub status: DatabaseStatus,
    pub port: u16,
    pub size: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume_name: Option<String>,
    pub env_vars: Vec<(String, String)>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateDatabaseRequest {
    pub name: String,
    #[serde(rename = "type")]
    pub db_type: String,
    pub version: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateDatabaseRequest {
    pub name: Option<String>,
    pub status: Option<DatabaseStatus>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg),
        }
    }
}
