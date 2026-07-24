//! API Key endpoints — desktop-local management of durable API keys
//! POST   /api/keys           create a new key (shown once)
//! GET    /api/keys           list keys (optionally ?database_id=...)
//! DELETE /api/keys/:id       revoke a key

use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
};
use tracing::{info, warn};

use crate::AppState;
use crate::auth::api_keys::{generate_api_key, generate_wire_password, hash_wire_password};
use crate::models::api_key::{
    CreateApiKeyRequest, CreateApiKeyResponse, ApiKeyInfo, ListApiKeysResponse,
};
use crate::sharing::share_store::ApiKeyRecord;

/// Default wire username when the caller doesn't provide a custom one:
/// "bennett_<slugified-name>" — e.g. "oshocks-backend" -> "bennett_oshocks_backend"
fn default_wire_username(name: &str) -> String {
    let slug: String = name.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    format!("bennett_{}", slug)
}

pub async fn create_api_key(
    State(state): State<AppState>,
    Json(req): Json<CreateApiKeyRequest>,
) -> Result<Json<crate::models::database::ApiResponse<CreateApiKeyResponse>>, StatusCode> {
    let db_exists = {
        let dbs = state.databases.lock().unwrap();
        dbs.iter().any(|d| d.id == req.database_id)
    };
    if !db_exists {
        return Ok(Json(crate::models::database::ApiResponse::error(
            format!("Database {} not found", req.database_id)
        )));
    }

    let (plaintext, key_hash) = generate_api_key();
    let permission = req.permission.unwrap_or_else(|| "ro".to_string());
    let tables = req.tables.unwrap_or_else(|| vec!["*".to_string()]);
    let id = uuid::Uuid::new_v4().to_string();
    let created_at = chrono::Utc::now();

    let max_rows = req.max_rows.unwrap_or(1000).clamp(1, 10000);
    let timeout_secs = req.timeout_secs.unwrap_or(30).clamp(1, 300);

    // Wire-protocol (MySQL/Postgres) credentials — optional, generated once,
    // shown to the caller exactly once, stored only as a hash thereafter.
    let (wire_username, wire_password_plaintext, wire_password_hash) =
        if req.enable_wire_access.unwrap_or(false) {
            let username = req.wire_username.clone().unwrap_or_else(|| default_wire_username(&req.name));
            let password = req.wire_password.clone().unwrap_or_else(generate_wire_password);
            let hash = hash_wire_password(&password);
            (Some(username), Some(password), Some(hash))
        } else {
            (None, None, None)
        };

    let record = ApiKeyRecord {
        id: id.clone(),
        key_hash: key_hash.clone(),
        db_id: req.database_id.clone(),
        name: req.name.clone(),
        permission: permission.clone(),
        tables: serde_json::to_string(&tables).unwrap_or_else(|_| r#"["*"]"#.to_string()),
        cols: req.cols.map(|c| c.to_string()),
        rls: req.rls,
        created_at,
        last_used_at: None,
        revoked: false,
        max_rows,
        timeout_secs,
        wire_username: wire_username.clone(),
        wire_password_hash: wire_password_hash.clone(),
    };

    if let Err(e) = state.share_store.create_api_key(&record).await {
        warn!("Failed to store API key: {}", e);
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Failed to store API key".to_string()
        )));
    }

    {
        let tunnel_lock = state.tunnel_tx.read().await;
        if let Some(ref tx) = *tunnel_lock {
            let msg = crate::sharing::relay::TunnelMessage::ApiKeyRegistered {
                key_hash: key_hash.clone(),
                db_id: req.database_id.clone(),
                permission: permission.clone(),
                wire_password_hash: wire_password_hash.clone(),
            };
            if let Err(e) = tx.send(msg) {
                warn!("Failed to notify relay about new API key: {}", e);
            } else {
                info!("Notified relay tunnel about new API key '{}'", req.name);
            }
        } else {
            warn!("Relay tunnel not connected — API key '{}' won't be reachable externally until reconnect", req.name);
        }
    }

    info!("Created API key '{}' for db {}", req.name, req.database_id);
    if wire_username.is_some() {
        info!("Wire-protocol access enabled for API key '{}' (username: {:?})", req.name, wire_username);
    }

    Ok(Json(crate::models::database::ApiResponse::success(CreateApiKeyResponse {
        id, key: plaintext, name: req.name, permission, created_at,
        wire_username,
        wire_password: wire_password_plaintext,
    })))
}

pub async fn list_api_keys(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<crate::models::database::ApiResponse<ListApiKeysResponse>>, StatusCode> {
    let dbs = { state.databases.lock().unwrap().clone() };
    let filter_db = params.get("database_id").cloned();

    let mut keys = Vec::new();
    for db in &dbs {
        if let Some(ref filter) = filter_db {
            if &db.id != filter { continue; }
        }
        match state.share_store.list_api_keys_by_db(&db.id).await {
            Ok(records) => {
                for r in records {
                    let tables: Vec<String> = serde_json::from_str(&r.tables)
                        .unwrap_or_else(|_| vec!["*".to_string()]);
                    keys.push(ApiKeyInfo {
                        id: r.id,
                        name: r.name,
                        db_id: r.db_id,
                        permission: r.permission,
                        tables,
                        created_at: r.created_at,
                        last_used_at: r.last_used_at,
                        revoked: r.revoked,
                        key_preview: format!("{}...{}", &r.key_hash[..6], &r.key_hash[r.key_hash.len()-4..]),
                        max_rows: r.max_rows,
                        timeout_secs: r.timeout_secs,
                        wire_enabled: r.wire_username.is_some(),
                        wire_username: r.wire_username,
                    });
                }
            }
            Err(e) => warn!("Failed to list API keys for db {}: {}", db.id, e),
        }
    }

    let total = keys.len();
    Ok(Json(crate::models::database::ApiResponse::success(ListApiKeysResponse { keys, total })))
}

/// DELETE /api/keys/:id/permanent — hard delete (irreversible)
pub async fn delete_api_key(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<crate::models::database::ApiResponse<serde_json::Value>>, StatusCode> {
    match state.share_store.hard_delete_api_key(&id).await {
        Ok(Some((key_hash, wire_password_hash))) => {
            state.rate_limiter.remove(&key_hash).await;

            let tunnel_lock = state.tunnel_tx.read().await;
            if let Some(ref tx) = *tunnel_lock {
                let msg = crate::sharing::relay::TunnelMessage::ApiKeyRevoked { key_hash, wire_password_hash };
                let _ = tx.send(msg);
                info!("Notified relay tunnel about deleted API key {}", id);
            }

            info!("Hard deleted API key {}", id);
            Ok(Json(crate::models::database::ApiResponse::success(serde_json::json!({
                "deleted": true,
                "id": id
            }))))
        }
        Ok(None) => Ok(Json(crate::models::database::ApiResponse::error(
            format!("API key {} not found", id)
        ))),
        Err(e) => {
            warn!("Failed to hard delete API key {}: {}", id, e);
            Ok(Json(crate::models::database::ApiResponse::error(
                "Failed to delete API key".to_string()
            )))
        }
    }
}

pub async fn revoke_api_key(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<crate::models::database::ApiResponse<serde_json::Value>>, StatusCode> {
    match state.share_store.revoke_api_key(&id).await {
        Ok(Some((key_hash, wire_password_hash))) => {
            state.rate_limiter.remove(&key_hash).await;

            let tunnel_lock = state.tunnel_tx.read().await;
            if let Some(ref tx) = *tunnel_lock {
                let msg = crate::sharing::relay::TunnelMessage::ApiKeyRevoked {
                    key_hash,
                    wire_password_hash,
                };
                let _ = tx.send(msg);
                info!("Notified relay tunnel about revoked API key {}", id);
            }
            Ok(Json(crate::models::database::ApiResponse::success(serde_json::json!({
                "revoked": true, "id": id
            }))))
        }
        Ok(None) => Ok(Json(crate::models::database::ApiResponse::error(
            format!("API key {} not found", id)
        ))),
        Err(e) => {
            warn!("Failed to revoke API key {}: {}", id, e);
            Ok(Json(crate::models::database::ApiResponse::error(
                "Failed to revoke API key".to_string()
            )))
        }
    }
}
