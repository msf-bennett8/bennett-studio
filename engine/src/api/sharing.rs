//! Share API endpoints
//! POST /api/shares — create share
//! GET /api/shares — list shares
//! DELETE /api/shares/:code — revoke share
//! POST /api/shares/:code/validate — validate share (guest)

use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
};
use chrono::Utc;
use tracing::{info, warn};

use crate::AppState;
use crate::models::share::{
    CreateShareRequest, CreateShareResponse, ShareLink, ShareStatus,
    ValidateShareRequest, ValidateShareResponse, RevokeShareRequest, ListSharesResponse,
};
use crate::auth::share_token::{SharePermission, build_share_url};
use crate::utils::bennett_code::generate_share_code;
use crate::utils::net::{detect_lan_ip, detect_engine_port};
use crate::sharing::share_store::ShareRecord;

/// Base URL for share links (configurable via env)
fn get_share_base_url() -> String {
    std::env::var("BENNETT_SHARE_BASE_URL")
        .unwrap_or_else(|_| "https://share-bennett-studio.vercel.app".to_string())
}

/// POST /api/shares — Create a new share link
pub async fn create_share(
    State(state): State<AppState>,
    Json(req): Json<CreateShareRequest>,
) -> Result<Json<crate::models::database::ApiResponse<CreateShareResponse>>, StatusCode> {
    // Find database
    let db = {
        let dbs = state.databases.lock().unwrap();
        dbs.iter().find(|d| d.id == req.database_id).cloned()
    };
    
    let db = match db {
        Some(d) => d,
        None => return Ok(Json(crate::models::database::ApiResponse::error(
            format!("Database {} not found", req.database_id)
        ))),
    };
    
    // Generate Bennett code
    let code = generate_share_code();
    
    // Determine permission
    let permission = req.permission.as_deref().unwrap_or("ro");
    let perm = SharePermission::from_str(permission);
    
    // Determine tables
    let tables = req.tables.unwrap_or_else(|| vec!["*".to_string()]);
    
    // Determine duration (default 24h)
    let duration = req.duration_hours.unwrap_or(24);
    let duration = duration.clamp(1, 168); // Max 7 days
    
    // Use stable host ID from store, or generate and persist
    let host_id = {
        let stored = state.share_store.get_host_id().await.ok().flatten();
        stored.unwrap_or_else(|| {
            let new_id = format!("host-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("unknown"));
            let _ = state.share_store.set_host_id(&new_id);
            new_id
        })
    };

    // Detect host network endpoint for guest resolution
    let host_ip = detect_lan_ip().unwrap_or_else(|| "127.0.0.1".to_string());
    let host_port = detect_engine_port();

    // Gather ICE candidates if P2P is enabled on this host
    // MUST happen before token creation so ICE can be embedded in JWT
    let ice_candidates = if std::env::var("BENNETT_ENABLE_P2P").is_ok() {
        match gather_engine_ice().await {
            Ok(ice) => {
                info!("P2P ICE gathered for share {}", code);
                Some(ice)
            }
            Err(e) => {
                warn!("P2P ICE gathering failed: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Create JWT token with embedded host endpoint + ICE candidates
    let token_manager = state.token_manager.read().await;
    let token_result = token_manager.create_token(
        code.clone(),
        db.id.clone(),
        host_id.clone(),
        Some(host_ip.clone()),
        Some(host_port),
        ice_candidates.clone(), // Embed ICE in JWT for self-contained URL
        perm.clone(),
        tables.clone(),
        req.cols.clone(),
        req.rls.clone(),
        duration,
    );

    let token = match token_result {
        Ok(t) => t,
        Err(e) => {
            warn!("Failed to create token: {}", e);
            return Ok(Json(crate::models::database::ApiResponse::error(
                "Failed to create share token".to_string()
            )));
        }
    };

    // Build share URL — ICE is now embedded in the JWT token itself
    // The URL is clean: https://share-bennett-studio.vercel.app/db/CODE?t=JWT
    // The JWT contains everything: host, port, ICE candidates, permissions, etc.
    let base_url = get_share_base_url();
    let url = build_share_url(&base_url, &code, &token.token);
    
    // Store in database
    let record = ShareRecord {
        code: code.clone(),
        db_id: db.id.clone(),
        host_id: host_id.clone(),
        host: Some(host_ip.clone()),
        port: Some(host_port),
        token_jti: token.jti.clone(),
        token: Some(token.token.clone()),
        permission: perm.as_str().to_string(),
        tables: serde_json::to_string(&tables).unwrap_or_else(|_| r#"["*"]"#.to_string()),
        cols: req.cols.map(|c| c.to_string()),
        rls: req.rls,
        created_at: Utc::now(),
        expires_at: token.expires_at,
        revoked: false,
        guest_count: 0,
        pinned: false,
        ice: ice_candidates.clone(),
    };
    
    if let Err(e) = state.share_store.create_share(&record).await {
        warn!("Failed to store share: {}", e);
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Failed to store share".to_string()
        )));
    }
    
    // Record host heartbeat immediately (host is alive since we're creating a share)
    let _ = state.share_store.record_heartbeat(&host_id, Some(host_ip.clone()), Some(host_port), env!("CARGO_PKG_VERSION")).await;

    // Notify relay tunnel about new share so relay can route external requests
    {
        let tunnel_lock = state.tunnel_tx.read().await;
        if let Some(ref tx) = *tunnel_lock {
            let msg = crate::sharing::relay::TunnelMessage::ShareCreated {
                code: code.clone(),
                db_id: db.id.clone(),
                permission: permission.to_string(),
                expires_at: token.expires_at.timestamp(),
            };
            if let Err(e) = tx.send(msg) {
                warn!("Failed to notify relay about share {}: {}", code, e);
            } else {
                info!("Notified relay tunnel about share {}", code);
            }
        } else {
            warn!("Relay tunnel not connected — share {} not registered with relay", code);
        }
    }

    info!("Created share {} for db {} with {} permission", code, db.name, permission);

    // Generate short signaling code for Firebase P2P
    let sig_code = if ice_candidates.is_some() {
        Some(format!("{}-{}", &code[..3], &code[3..6]))
    } else {
        None
    };

    Ok(Json(crate::models::database::ApiResponse::success(CreateShareResponse {
        code: code.clone(),
        url,
        token: token.token,
        expires_at: token.expires_at,
        ice: ice_candidates,
        sig: sig_code,
    })))
}

/// GET /api/shares — List active shares
pub async fn list_shares(
    State(state): State<AppState>,
) -> Result<Json<crate::models::database::ApiResponse<ListSharesResponse>>, StatusCode> {
    // Get all databases to build response
    let dbs = {
        let dbs = state.databases.lock().unwrap();
        dbs.clone()
    };
    
    let mut all_shares = Vec::new();
    
    for db in &dbs {
        match state.share_store.list_all_shares_by_db(&db.id).await {
            Ok(shares) => {
                for record in shares {
                    let status = if record.revoked {
                        ShareStatus::Revoked
                    } else if record.expires_at < Utc::now() {
                        ShareStatus::Expired
                    } else {
                        ShareStatus::Active
                    };
                    
                    let tables: Vec<String> = serde_json::from_str(&record.tables)
                        .unwrap_or_else(|_| vec!["*".to_string()]);
                    
                    let code = record.code.clone();
                    // Use stored token if available, otherwise placeholder
                    let token = record.token.as_deref().unwrap_or("...");
                    let url = build_share_url(&get_share_base_url(), &code, token);
                    all_shares.push(ShareLink {
                        code: record.code,
                        url,
                        db_id: record.db_id,
                        db_name: db.name.clone(),
                        db_type: db.db_type.clone(),
                        permission: record.permission,
                        tables,
                        expires_at: record.expires_at,
                        created_at: record.created_at,
                        guest_count: record.guest_count,
                        pinned: record.pinned,
                        status,
                        ice: record.ice.clone(), // Pass through ICE if stored
                    });
                }
            }
            Err(e) => {
                warn!("Failed to list shares for db {}: {}", db.id, e);
            }
        }
    }
    
    let total = all_shares.len();
    
    Ok(Json(crate::models::database::ApiResponse::success(ListSharesResponse {
        shares: all_shares,
        total,
    })))
}

/// POST /api/shares/:code/pin — Toggle pin status for a share
pub async fn toggle_pin_share(
    Path(code): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<crate::models::database::ApiResponse<serde_json::Value>>, StatusCode> {
    match state.share_store.toggle_pin_share(&code).await {
        Ok(true) => {
            info!("Toggled pin for share {}", code);
            Ok(Json(crate::models::database::ApiResponse::success(serde_json::json!({
                "pinned": true,
                "code": code
            }))))
        }
        Ok(false) => {
            Ok(Json(crate::models::database::ApiResponse::error(
                format!("Share {} not found", code)
            )))
        }
        Err(e) => {
            warn!("Failed to toggle pin for share {}: {}", code, e);
            Ok(Json(crate::models::database::ApiResponse::error(
                "Failed to toggle pin".to_string()
            )))
        }
    }
}

/// GET /api/shares/:code/schema — Get schema for a shared database (guest access)
pub async fn get_share_schema(
    Path(code): Path<String>,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
) -> Result<Json<crate::models::database::ApiResponse<serde_json::Value>>, StatusCode> {
    // Extract token from header
    let token = headers
        .get("x-share-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if token.is_empty() {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Missing X-Share-Token header".to_string()
        )));
    }

    // Get share record
    let record = match state.share_store.get_share(&code).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Ok(Json(crate::models::database::ApiResponse::error(
                "Share not found".to_string()
            )));
        }
        Err(e) => {
            warn!("Failed to get share {}: {}", code, e);
            return Ok(Json(crate::models::database::ApiResponse::error(
                "Internal error".to_string()
            )));
        }
    };

    // Check if revoked
    if record.revoked {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Share has been revoked".to_string()
        )));
    }

    // Check expiration
    if record.expires_at < Utc::now() {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Share has expired".to_string()
        )));
    }

    // Validate JWT token
    let token_manager = state.token_manager.read().await;
    let validated = match token_manager.validate_token(token) {
        Ok(v) => v,
        Err(e) => {
            return Ok(Json(crate::models::database::ApiResponse::error(
                format!("Invalid token: {}", e)
            )));
        }
    };

    // Verify token matches code
    if validated.code != code {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Token does not match share code".to_string()
        )));
    }

    // Check if token JTI is revoked
    if state.share_store.is_revoked(&validated.jti).await {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Token has been revoked".to_string()
        )));
    }

    // Check host heartbeat with self-healing
    let host_alive = match state.share_store.is_host_alive(&record.host_id).await {
        Ok(alive) => alive,
        Err(e) => {
            tracing::warn!("Heartbeat check failed for share {} schema: {}, assuming alive", code, e);
            true
        }
    };

    if !host_alive {
        tracing::info!("Host {} for share {} appears offline, attempting self-healing", record.host_id, code);
        
        match state.share_store.record_heartbeat(
            &record.host_id,
            record.host.clone(),
            record.port,
            env!("CARGO_PKG_VERSION")
        ).await {
            Ok(_) => tracing::info!("Self-healing heartbeat recorded for host {}", record.host_id),
            Err(e) => tracing::warn!("Self-healing heartbeat failed: {}", e),
        }

        let host_alive_after = match state.share_store.is_host_alive(&record.host_id).await {
            Ok(alive) => alive,
            Err(_) => true,
        };
        
        if !host_alive_after {
            return Ok(Json(crate::models::database::ApiResponse::error(
                "Host is currently offline. Please try again later.".to_string()
            )));
        }
    }

    // Get database instance
    let instance = {
        let db = state.databases.lock().unwrap();
        match db.iter().find(|d| d.id == record.db_id).cloned() {
            Some(i) => i,
            None => {
                return Ok(Json(crate::models::database::ApiResponse::error(
                    "Database not found".to_string()
                )));
            }
        }
    };

    // Auto-connect if not connected
    {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&record.db_id) {
            if let Err(e) = conn.connect(&instance).await {
                return Ok(Json(crate::models::database::ApiResponse::error(
                    format!("Connection failed: {}", e)
                )));
            }
        }
    }

    // Fetch schema
    let result = {
        let conn = state.connections.lock().await;
        let db_type = {
            let dbs = state.databases.lock().unwrap();
            dbs.iter().find(|d| d.id == record.db_id)
                .map(|d| d.db_type.clone())
                .unwrap_or_else(|| "unknown".to_string())
        };
        match conn.get_schema(&record.db_id).await {
            Ok(tables) => {
                let response = serde_json::json!({
                    "tables": tables,
                    "databaseName": instance.name,
                    "databaseType": db_type,
                    "databaseVersion": instance.version,
                });
                Json(crate::models::database::ApiResponse::success(response))
            },
            Err(e) => Json(crate::models::database::ApiResponse::error(
                format!("Schema query failed: {}", e)
            )),
        }
    };

    Ok(result)
}

/// DELETE /api/shares/:code/permanent — Hard delete a share (permanent removal)
pub async fn delete_share(
    Path(code): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<crate::models::database::ApiResponse<serde_json::Value>>, StatusCode> {
    match state.share_store.hard_delete_share(&code).await {
        Ok(true) => {
            // Notify relay tunnel about deleted share
            {
                let tunnel_lock = state.tunnel_tx.read().await;
                if let Some(ref tx) = *tunnel_lock {
                    let msg = crate::sharing::relay::TunnelMessage::ShareRevoked {
                        code: code.clone(),
                    };
                    let _ = tx.send(msg);
                    info!("Notified relay tunnel about deleted share {}", code);
                }
            }

            info!("Hard deleted share {}", code);
            Ok(Json(crate::models::database::ApiResponse::success(serde_json::json!({
                "deleted": true,
                "code": code
            }))))
        }
        Ok(false) => {
            Ok(Json(crate::models::database::ApiResponse::error(
                format!("Share {} not found", code)
            )))
        }
        Err(e) => {
            warn!("Failed to hard delete share {}: {}", code, e);
            Ok(Json(crate::models::database::ApiResponse::error(
                "Failed to delete share".to_string()
            )))
        }
    }
}

/// DELETE /api/shares/:code — Revoke a share
pub async fn revoke_share(
    Path(code): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<RevokeShareRequest>,
) -> Result<Json<crate::models::database::ApiResponse<serde_json::Value>>, StatusCode> {
    let reason = req.reason.as_deref().unwrap_or("host_revoked");
    
    match state.share_store.revoke_share(&code, reason).await {
        Ok(true) => {
            // Notify relay tunnel about revoked share
            {
                let tunnel_lock = state.tunnel_tx.read().await;
                if let Some(ref tx) = *tunnel_lock {
                    let msg = crate::sharing::relay::TunnelMessage::ShareRevoked {
                        code: code.clone(),
                    };
                    let _ = tx.send(msg);
                    info!("Notified relay tunnel about revoked share {}", code);
                }
            }

            info!("Revoked share {}", code);
            Ok(Json(crate::models::database::ApiResponse::success(serde_json::json!({
                "revoked": true,
                "code": code
            }))))
        }
        Ok(false) => {
            Ok(Json(crate::models::database::ApiResponse::error(
                format!("Share {} not found", code)
            )))
        }
        Err(e) => {
            warn!("Failed to revoke share {}: {}", code, e);
            Ok(Json(crate::models::database::ApiResponse::error(
                "Failed to revoke share".to_string()
            )))
        }
    }
}

/// POST /api/shares/:code/validate — Validate a share (guest)
pub async fn validate_share(
    Path(code): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<ValidateShareRequest>,
) -> Result<Json<crate::models::database::ApiResponse<ValidateShareResponse>>, StatusCode> {
    // Get share record
    let record = match state.share_store.get_share(&code).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Ok(Json(crate::models::database::ApiResponse::error(
                "Share not found".to_string()
            )));
        }
        Err(e) => {
            warn!("Failed to get share {}: {}", code, e);
            return Ok(Json(crate::models::database::ApiResponse::error(
                "Internal error".to_string()
            )));
        }
    };
    
    // Check if revoked
    if record.revoked {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Share has been revoked".to_string()
        )));
    }
    
    // Check expiration
    if record.expires_at < Utc::now() {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Share has expired".to_string()
        )));
    }
    
    // Validate JWT token
    let token_manager = state.token_manager.read().await;
    let validated = match token_manager.validate_token(&req.token) {
        Ok(v) => v,
        Err(e) => {
            return Ok(Json(crate::models::database::ApiResponse::error(
                format!("Invalid token: {}", e)
            )));
        }
    };
    
    // Verify token matches code
    if validated.code != code {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Token does not match share code".to_string()
        )));
    }
    
    // Check if token JTI is revoked
    if state.share_store.is_revoked(&validated.jti).await {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Token has been revoked".to_string()
        )));
    }
    
    // Get database info
    let _db_name = {
        let dbs = state.databases.lock().unwrap();
        dbs.iter().find(|d| d.id == record.db_id)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| "Unknown".to_string())
    };
    
    let tables: Vec<String> = serde_json::from_str(&record.tables)
        .unwrap_or_else(|_| vec!["*".to_string()]);
    
    // Check host heartbeat with self-healing
    let host_alive = match state.share_store.is_host_alive(&record.host_id).await {
        Ok(alive) => alive,
        Err(e) => {
            tracing::warn!("Heartbeat check failed for share {}: {}, assuming alive", code, e);
            true // If heartbeat table doesn't exist or errors, assume alive
        }
    };

    if !host_alive {
        tracing::info!("Host {} for share {} appears offline, attempting self-healing", record.host_id, code);
        
        // Self-healing: record heartbeat for this host
        match state.share_store.record_heartbeat(
            &record.host_id,
            record.host.clone(),
            record.port,
            env!("CARGO_PKG_VERSION")
        ).await {
            Ok(_) => {
                tracing::info!("Self-healing heartbeat recorded for host {}", record.host_id);
            }
            Err(e) => {
                tracing::warn!("Self-healing heartbeat failed for host {}: {}", record.host_id, e);
            }
        }

        // Re-check after self-healing
        let host_alive_after = match state.share_store.is_host_alive(&record.host_id).await {
            Ok(alive) => alive,
            Err(_) => true, // If check fails, be lenient
        };
        
        if !host_alive_after {
            return Ok(Json(crate::models::database::ApiResponse::error(
                "Host is currently offline. Please try again later.".to_string()
            )));
        }
    }

    info!("Validated share {} for guest", code);

    Ok(Json(crate::models::database::ApiResponse::success(ValidateShareResponse {
        valid: true,
        code: code.clone(),
        db_id: record.db_id,
        permission: record.permission,
        tables,
        expires_at: record.expires_at,
        host_online: true,
    })))
}

/// GET /api/shares/:code — Get share info (public, no auth needed)
pub async fn get_share_info(
    Path(code): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<crate::models::database::ApiResponse<serde_json::Value>>, StatusCode> {
    let record = match state.share_store.get_share(&code).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Ok(Json(crate::models::database::ApiResponse::error(
                "Share not found".to_string()
            )));
        }
        Err(e) => {
            warn!("Failed to get share {}: {}", code, e);
            return Ok(Json(crate::models::database::ApiResponse::error(
                "Internal error".to_string()
            )));
        }
    };
    
    // Don't expose sensitive info publicly
    Ok(Json(crate::models::database::ApiResponse::success(serde_json::json!({
        "code": record.code,
        "db_id": record.db_id,
        "permission": record.permission,
        "tables": serde_json::from_str::<Vec<String>>(&record.tables).unwrap_or_else(|_| vec!["*".to_string()]),
        "expires_at": record.expires_at,
        "status": if record.revoked { "revoked" } else if record.expires_at < Utc::now() { "expired" } else { "active" },
        "guest_count": record.guest_count,
    })) ))
}

/// POST /api/shares/:code/query — Execute a query on a shared database (guest)
pub async fn execute_share_query(
    Path(code): Path<String>,
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(req): Json<crate::api::http::ExecuteQueryRequest>,
) -> Result<Json<crate::models::database::ApiResponse<crate::control_plane::connection::manager::QueryResult>>, StatusCode> {
    // Extract token from header
    let token = headers
        .get("x-share-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if token.is_empty() {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Missing X-Share-Token header".to_string()
        )));
    }

    // Get share record
    let record = match state.share_store.get_share(&code).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Ok(Json(crate::models::database::ApiResponse::error(
                "Share not found".to_string()
            )));
        }
        Err(e) => {
            warn!("Failed to get share {}: {}", code, e);
            return Ok(Json(crate::models::database::ApiResponse::error(
                "Internal error".to_string()
            )));
        }
    };

    // Check if revoked
    if record.revoked {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Share has been revoked".to_string()
        )));
    }

    // Check expiration
    if record.expires_at < Utc::now() {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Share has expired".to_string()
        )));
    }

    // Validate JWT token
    let token_manager = state.token_manager.read().await;
    let validated = match token_manager.validate_token(token) {
        Ok(v) => v,
        Err(e) => {
            return Ok(Json(crate::models::database::ApiResponse::error(
                format!("Invalid token: {}", e)
            )));
        }
    };

    // Verify token matches code
    if validated.code != code {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Token does not match share code".to_string()
        )));
    }

    // Check if token JTI is revoked
    if state.share_store.is_revoked(&validated.jti).await {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Token has been revoked".to_string()
        )));
    }

    // Validate SQL (same rules as internal query)
    if let Err(e) = crate::api::http::validate_sql(&req.sql) {
        return Ok(Json(crate::models::database::ApiResponse::error(e)));
    }

    // Get database instance
    let instance = {
        let db = state.databases.lock().unwrap();
        match db.iter().find(|d| d.id == record.db_id).cloned() {
            Some(i) => i,
            None => {
                return Ok(Json(crate::models::database::ApiResponse::error(
                    "Database not found".to_string()
                )));
            }
        }
    };

    // Auto-connect if not connected
    {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&record.db_id) {
            if let Err(e) = conn.connect(&instance).await {
                return Ok(Json(crate::models::database::ApiResponse::error(
                    format!("Connection failed: {}", e)
                )));
            }
        }
    }

    // Execute query
    let result = {
        let conn = state.connections.lock().await;
        match conn.execute(&record.db_id, &req.sql).await {
            Ok(r) => Json(crate::models::database::ApiResponse::success(r)),
            Err(e) => Json(crate::models::database::ApiResponse::error(
                format!("Query failed: {}", e)
            )),
        }
    };

    Ok(result)
}

/// GET /api/shares/:code/resolve — Resolve share code to host endpoint (guest)
/// Returns the host IP and port so the guest can connect directly
pub async fn resolve_share(
    Path(code): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<crate::models::database::ApiResponse<serde_json::Value>>, StatusCode> {
    // Get share record
    let record = match state.share_store.get_share(&code).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return Ok(Json(crate::models::database::ApiResponse::error(
                "Share not found".to_string()
            )));
        }
        Err(e) => {
            warn!("Failed to get share {} for resolve: {}", code, e);
            return Ok(Json(crate::models::database::ApiResponse::error(
                "Internal error".to_string()
            )));
        }
    };

    // Check if revoked or expired
    if record.revoked {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Share has been revoked".to_string()
        )));
    }
    if record.expires_at < Utc::now() {
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Share has expired".to_string()
        )));
    }

    // Read host/port from stored record
    let host_ip = record.host.unwrap_or_else(|| "127.0.0.1".to_string());
    let host_port = record.port.unwrap_or(3001);

    info!("Resolved share {} to {}:{}", code, host_ip, host_port);

    Ok(Json(crate::models::database::ApiResponse::success(serde_json::json!({
        "code": record.code,
        "host": host_ip,
        "port": host_port,
        "base_url": format!("http://{}:{}", host_ip, host_port),
        "ttl_seconds": 300,
    })) ))
}

/// Default Firebase Realtime Database URL for P2P signaling
/// Spark plan = free forever, no credit card needed
const DEFAULT_FIREBASE_URL: &str = "https://bennett-p2p-signaling-default-rtdb.europe-west1.firebasedatabase.app/";

/// Gather ICE candidates from the relay process
/// Returns URL-safe base64-encoded ICE string
async fn gather_engine_ice() -> Result<String, String> {
    // Try to find relay binary
    let relay_path = std::env::var("BENNETT_RELAY_PATH")
        .unwrap_or_else(|_| "./target/release/bennett-relay".to_string());

    let output = tokio::process::Command::new(&relay_path)
        .arg("--gather-ice")
        .output()
        .await
        .map_err(|e| format!("Failed to run relay: {}", e))?;

    if !output.status.success() {
        return Err(format!("Relay ICE gathering failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    // Relay outputs URL-safe base64-encoded ICE candidates (no padding)
    let b64 = String::from_utf8_lossy(&output.stdout).trim().to_string();
    
    // Validate it's valid base64 using base64 0.22 API
    use base64::Engine;
    let _ = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&b64)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(&b64))
        .or_else(|_| base64::engine::general_purpose::STANDARD.decode(&b64))
        .map_err(|e| format!("Relay output is not valid base64: {}", e))?;

    Ok(b64)
}

/// Get Firebase Realtime Database URL for P2P signaling
pub fn get_firebase_url() -> String {
    std::env::var("BENNETT_FIREBASE_URL")
        .unwrap_or_else(|_| DEFAULT_FIREBASE_URL.to_string())
}
