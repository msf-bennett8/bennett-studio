//! GET /api/engine/info — real host identity + runtime facts for the
//! General settings page (host_id, data dir, relay URL, live counts).

use axum::{extract::State, Json};
use crate::AppState;

#[derive(serde::Serialize)]
pub struct EngineInfo {
    pub host_id: String,
    pub version: String,
    pub relay_url: String,
    pub data_dir: String,
    pub database_count: usize,
    pub active_share_count: usize,
}

pub async fn get_engine_info(
    State(state): State<AppState>,
) -> Json<crate::models::database::ApiResponse<EngineInfo>> {
    let host_id = state.share_store.get_host_id().await.ok().flatten()
        .unwrap_or_else(|| "unassigned".to_string());
    let relay_url = std::env::var("BENNETT_RELAY_URL")
        .unwrap_or_else(|_| "not configured".to_string());
    let data_dir = dirs::home_dir()
        .map(|p| p.join(".bennett").join("data").to_string_lossy().to_string())
        .unwrap_or_default();
    let database_count = state.databases.lock().unwrap().len();
    let active_share_count = state.share_store.list_all_active().await.map(|s| s.len()).unwrap_or(0);

    Json(crate::models::database::ApiResponse::success(EngineInfo {
        host_id,
        version: env!("CARGO_PKG_VERSION").to_string(),
        relay_url,
        data_dir,
        database_count,
        active_share_count,
    }))
}
