use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: String,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub pinned: bool,
    pub archived: bool,
    pub color: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub word_count: i64,
    pub char_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateNoteRequest {
    pub title: String,
    pub content: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateNoteRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
    pub color: Option<String>,
    pub pinned: Option<bool>,
    pub archived: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SyncNotesRequest {
    pub notes: Vec<Note>,
    pub device_id: String,
    pub last_sync: Option<DateTime<Utc>>,
}

fn default_color() -> String {
    "#00d4aa".to_string()
}

impl Note {
    pub fn word_count(content: &str) -> i64 {
        content.trim().split_whitespace().count() as i64
    }

    pub fn char_count(content: &str) -> i64 {
        content.len() as i64
    }
}
