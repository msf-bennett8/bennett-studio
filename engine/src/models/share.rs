use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareSession {
    pub id: String,
    pub database_id: String,
    pub token: String,
    pub expires_at: String,
    pub read_only: bool,
}
