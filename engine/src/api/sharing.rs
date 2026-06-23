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
        .unwrap_or_else(|_| "https://share.bennett.studio".to_string())
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
    
    // Generate host ID (fingerprint)
    let host_id = format!("host-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("unknown"));

    // Detect host network endpoint for guest resolution
    let host_ip = detect_lan_ip().unwrap_or_else(|| "127.0.0.1".to_string());
    let host_port = detect_engine_port();

    // Create JWT token with embedded host endpoint
    let token_manager = state.token_manager.read().await;
    let token_result = token_manager.create_token(
        code.clone(),
        db.id.clone(),
        host_id.clone(),
        Some(host_ip.clone()),
        Some(host_port),
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
    
    // Build share URL
    let base_url = get_share_base_url();
    let url = build_share_url(&base_url, &code, &token.token);
    
    // Store in database
    let record = ShareRecord {
        code: code.clone(),
        db_id: db.id.clone(),
        host_id,
        host: Some(host_ip.clone()),
        port: Some(host_port),
        token_jti: token.jti.clone(),
        permission: perm.as_str().to_string(),
        tables: serde_json::to_string(&tables).unwrap_or_else(|_| r#"["*"]"#.to_string()),
        cols: req.cols.map(|c| c.to_string()),
        rls: req.rls,
        created_at: Utc::now(),
        expires_at: token.expires_at,
        revoked: false,
        guest_count: 0,
    };
    
    if let Err(e) = state.share_store.create_share(&record).await {
        warn!("Failed to store share: {}", e);
        return Ok(Json(crate::models::database::ApiResponse::error(
            "Failed to store share".to_string()
        )));
    }
    
    info!("Created share {} for db {} with {} permission", code, db.name, permission);
    
    Ok(Json(crate::models::database::ApiResponse::success(CreateShareResponse {
        code: code.clone(),
        url,
        token: token.token,
        expires_at: token.expires_at,
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
        match state.share_store.list_shares_by_db(&db.id).await {
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
                    all_shares.push(ShareLink {
                        code: record.code,
                        url: build_share_url(&get_share_base_url(), &code, "..."),
                        db_id: record.db_id,
                        db_name: db.name.clone(),
                        db_type: db.db_type.clone(),
                        permission: record.permission,
                        tables,
                        expires_at: record.expires_at,
                        created_at: record.created_at,
                        guest_count: record.guest_count,
                        status,
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

/// DELETE /api/shares/:code — Revoke a share
pub async fn revoke_share(
    Path(code): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<RevokeShareRequest>,
) -> Result<Json<crate::models::database::ApiResponse<serde_json::Value>>, StatusCode> {
    let reason = req.reason.as_deref().unwrap_or("host_revoked");
    
    match state.share_store.revoke_share(&code, reason).await {
        Ok(true) => {
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
