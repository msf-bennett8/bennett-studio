//! GET /api/activity — recent audit log entries for the Activity settings page

use axum::{extract::{State, Query}, Json};
use std::collections::HashMap;
use crate::AppState;
use crate::audit::AuditEntry;

pub async fn list_activity(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Json<crate::models::database::ApiResponse<Vec<AuditEntry>>> {
    let limit: i64 = params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(100).clamp(1, 1000);

    match &state.audit_service {
        Some(audit) => match audit.query(None, None, None, None, limit).await {
            Ok(entries) => Json(crate::models::database::ApiResponse::success(entries)),
            Err(e) => Json(crate::models::database::ApiResponse::error(format!("Failed to fetch activity: {}", e))),
        },
        None => Json(crate::models::database::ApiResponse::success(vec![])),
    }
}

pub async fn clear_activity(
    State(state): State<AppState>,
) -> Json<crate::models::database::ApiResponse<serde_json::Value>> {
    match &state.audit_service {
        Some(audit) => match audit.clear_all().await {
            Ok(count) => Json(crate::models::database::ApiResponse::success(serde_json::json!({ "cleared": count }))),
            Err(e) => Json(crate::models::database::ApiResponse::error(format!("Failed to clear activity: {}", e))),
        },
        None => Json(crate::models::database::ApiResponse::success(serde_json::json!({ "cleared": 0 }))),
    }
}
