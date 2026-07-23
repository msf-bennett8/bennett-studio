//! GET /api/guests, DELETE /api/guests/:id — connected guest sessions
//! for the Guests settings page

use axum::{extract::{State, Path}, Json};
use crate::AppState;
use crate::sharing::share_store::GuestSession;

pub async fn list_guests(
    State(state): State<AppState>,
) -> Json<crate::models::database::ApiResponse<Vec<GuestSession>>> {
    match state.share_store.list_all_guest_sessions().await {
        Ok(sessions) => Json(crate::models::database::ApiResponse::success(sessions)),
        Err(e) => Json(crate::models::database::ApiResponse::error(format!("Failed to fetch guests: {}", e))),
    }
}

pub async fn disconnect_guest(
    Path(session_id): Path<String>,
    State(state): State<AppState>,
) -> Json<crate::models::database::ApiResponse<serde_json::Value>> {
    match state.share_store.record_guest_disconnect(&session_id).await {
        Ok(_) => Json(crate::models::database::ApiResponse::success(serde_json::json!({ "disconnected": true }))),
        Err(e) => Json(crate::models::database::ApiResponse::error(format!("Failed to disconnect guest: {}", e))),
    }
}
