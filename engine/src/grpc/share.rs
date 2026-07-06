//! gRPC ShareService implementation
//! Maps to the same business logic as REST/Connect-RPC in api/sharing.rs

use tonic::{Request, Response, Status};
use tracing::{info, warn};

use crate::AppState;
use crate::grpc::generated::{
    share_service_server::ShareService,
    CreateShareRequest, CreateShareResponse, ListSharesRequest, ListSharesResponse,
    RevokeShareRequest, RevokeShareResponse, ValidateShareRequest, ValidateShareResponse,
    ShareLink, GetShareInfoRequest, ShareInfo,
};
use crate::utils::bennett_code::generate_share_code;
use crate::auth::share_token::{SharePermission, build_share_url};
use crate::sharing::share_store::ShareRecord;

fn get_share_base_url() -> String {
    std::env::var("BENNETT_SHARE_BASE_URL")
        .unwrap_or_else(|_| "https://share.bennett.studio".to_string())
}
use chrono::Utc;

pub struct ShareGrpcService {
    state: AppState,
}

impl ShareGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl ShareService for ShareGrpcService {
    async fn create_share(
        &self,
        request: Request<CreateShareRequest>,
    ) -> Result<Response<CreateShareResponse>, Status> {
        let req = request.into_inner();
        
        // Find database
        let db = {
            let dbs = self.state.databases.lock().unwrap();
            dbs.iter().find(|d| d.id == req.database_id).cloned()
        };
        
        let db = db.ok_or_else(|| Status::not_found("Database not found"))?;
        
        // Generate code
        let code = generate_share_code();
        let permission = if req.permission.is_empty() { "ro" } else { &req.permission };
        let perm = SharePermission::from_str(permission);
        let perm_str = perm.as_str().to_string();
        let tables = if req.tables.is_empty() { vec!["*".to_string()] } else { req.tables.clone() };
        let duration = req.duration_hours.clamp(1, 168);
        let host_id = format!("host-{}", uuid::Uuid::new_v4().to_string().split('-').next().unwrap_or("unknown"));

        // Create JWT
        let token_manager = self.state.token_manager.read().await;
        let token_result = token_manager.create_token(
            code.clone(),
            db.id.clone(),
            host_id.clone(),
            None, // host
            None, // port
            None, // ice (gRPC path doesn't do P2P ICE gathering yet)
            perm,
            tables.clone(),
            None, // cols
            if req.rls.is_empty() { None } else { Some(req.rls.clone()) },
            duration,
        ).map_err(|e| Status::internal(format!("Token creation failed: {}", e)))?;
        
        // Build URL
        let base_url = std::env::var("BENNETT_SHARE_BASE_URL")
            .unwrap_or_else(|_| "https://share.bennett.studio".to_string());
        let url = build_share_url(&base_url, &code, &token_result.token);
        
        // Store in DB
        let record = ShareRecord {
            code: code.clone(),
            db_id: db.id.clone(),
            host_id,
            host: None,
            port: None,
            token_jti: token_result.jti.clone(),
            token: Some(token_result.token.clone()),
            permission: perm_str,
            tables: serde_json::to_string(&tables).unwrap_or_else(|_| r#"["*"]"#.to_string()),
            cols: None,
            rls: if req.rls.is_empty() { None } else { Some(req.rls) },
            created_at: Utc::now(),
            expires_at: token_result.expires_at,
            revoked: false,
            guest_count: 0,
            pinned: false,
            ice: None, // gRPC path doesn't do P2P ICE gathering yet
        };
        
        self.state.share_store.create_share(&record).await
            .map_err(|e| Status::internal(format!("Store failed: {}", e)))?;
        
        info!("gRPC: Created share {} for db {}", code, db.name);
        
        Ok(Response::new(CreateShareResponse {
            code,
            url,
            token: token_result.token,
            expires_at: token_result.expires_at.to_rfc3339(),
            ice: String::new(), // Empty string for gRPC (no P2P in gRPC path yet)
        }))
    }

    async fn list_shares(
        &self,
        _request: Request<ListSharesRequest>,
    ) -> Result<Response<ListSharesResponse>, Status> {
        let dbs = {
            let dbs = self.state.databases.lock().unwrap();
            dbs.clone()
        };
        
        let mut all_shares = Vec::new();
        
        for db in &dbs {
            match self.state.share_store.list_shares_by_db(&db.id).await {
                Ok(shares) => {
                    for record in shares {
                        let status = if record.revoked {
                            "revoked".to_string()
                        } else if record.expires_at < Utc::now() {
                            "expired".to_string()
                        } else {
                            "active".to_string()
                        };

                        let tables: Vec<String> = serde_json::from_str(&record.tables)
                            .unwrap_or_else(|_| vec!["*".to_string()]);

                        let code = record.code.clone();
                        all_shares.push(ShareLink {
                            code: code.clone(),
                            url: build_share_url(&get_share_base_url(), &code, "..."),
                            db_id: record.db_id,
                            db_name: db.name.clone(),
                            db_type: db.db_type.clone(),
                            permission: record.permission,
                            tables,
                            expires_at: record.expires_at.to_rfc3339(),
                            created_at: record.created_at.to_rfc3339(),
                            guest_count: record.guest_count,
                            pinned: record.pinned,
                            status,
                        });
                    }
                }
                Err(e) => {
                    warn!("Failed to list shares: {}", e);
                }
            }
        }

        let total = all_shares.len() as i32;
        Ok(Response::new(ListSharesResponse {
            shares: all_shares,
            total,
        }))
    }

    async fn revoke_share(
        &self,
        request: Request<RevokeShareRequest>,
    ) -> Result<Response<RevokeShareResponse>, Status> {
        let req = request.into_inner();
        
        let success = self.state.share_store.revoke_share(&req.code, &req.reason)
            .await
            .map_err(|e| Status::internal(format!("Revoke failed: {}", e)))?;
        
        if !success {
            return Err(Status::not_found("Share not found"));
        }
        
        info!("gRPC: Revoked share {}", req.code);
        
        Ok(Response::new(RevokeShareResponse {
            revoked: true,
            code: req.code,
        }))
    }

    async fn validate_share(
        &self,
        request: Request<ValidateShareRequest>,
    ) -> Result<Response<ValidateShareResponse>, Status> {
        let req = request.into_inner();
        
        // Get share record
        let record = self.state.share_store.get_share(&req.code).await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found("Share not found"))?;
        
        if record.revoked {
            return Err(Status::permission_denied("Share has been revoked"));
        }
        
        if record.expires_at < Utc::now() {
            return Err(Status::permission_denied("Share has expired"));
        }
        
        // Validate JWT
        let token_manager = self.state.token_manager.read().await;
        let validated = token_manager.validate_token(&req.token)
            .map_err(|e| Status::unauthenticated(format!("Invalid token: {}", e)))?;
        
        if validated.code != req.code {
            return Err(Status::unauthenticated("Token does not match share code"));
        }
        
        if self.state.share_store.is_revoked(&validated.jti).await {
            return Err(Status::permission_denied("Token has been revoked"));
        }
        
        let tables: Vec<String> = serde_json::from_str(&record.tables)
            .unwrap_or_else(|_| vec!["*".to_string()]);
        
        Ok(Response::new(ValidateShareResponse {
            valid: true,
            code: req.code,
            db_id: record.db_id,
            permission: record.permission,
            tables,
            expires_at: record.expires_at.to_rfc3339(),
            host_online: true,
        }))
    }

    async fn get_share_info(
        &self,
        request: Request<GetShareInfoRequest>,
    ) -> Result<Response<ShareInfo>, Status> {
        let req = request.into_inner();
        
        let record = self.state.share_store.get_share(&req.code).await
            .map_err(|e| Status::internal(format!("Database error: {}", e)))?
            .ok_or_else(|| Status::not_found("Share not found"))?;
        
        let tables: Vec<String> = serde_json::from_str(&record.tables)
            .unwrap_or_else(|_| vec!["*".to_string()]);
        
        let status = if record.revoked {
            "revoked".to_string()
        } else if record.expires_at < Utc::now() {
            "expired".to_string()
        } else {
            "active".to_string()
        };
        
        Ok(Response::new(ShareInfo {
            code: record.code,
            db_id: record.db_id,
            permission: record.permission,
            tables,
            expires_at: record.expires_at.to_rfc3339(),
            status,
            guest_count: record.guest_count,
        }))
    }
}
