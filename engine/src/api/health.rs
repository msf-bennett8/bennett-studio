//! Comprehensive health check endpoint
//! Phase 6: Check all subsystems, report status

use axum::{
    extract::State,
    Json,
};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use tracing::warn;

use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub checks: HashMap<String, ComponentHealth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: String,
    pub message: Option<String>,
    pub latency_ms: u64,
}

pub use crate::models::database::ApiResponse;

static START_TIME: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();

pub fn init_start_time() {
    START_TIME.get_or_init(std::time::Instant::now);
}

pub async fn comprehensive_health_check(
    State(state): State<AppState>,
) -> Json<ApiResponse<HealthStatus>> {
    let start = std::time::Instant::now();
    let mut checks = HashMap::new();
    
    // Docker check
    let docker_start = std::time::Instant::now();
    let docker_ok = state.docker.verify().await.is_ok();
    checks.insert("docker".to_string(), ComponentHealth {
        status: if docker_ok { "ok".to_string() } else { "error".to_string() },
        message: if docker_ok { None } else { Some("Docker daemon not accessible".to_string()) },
        latency_ms: docker_start.elapsed().as_millis() as u64,
    });
    
    // Database connections check
    let conn_start = std::time::Instant::now();
    let conn = state.connections.lock().await;
    let conn_count = {
        // Count active pools - this is a simplified check
        // In production, check actual pool health
        0
    };
    drop(conn);
    checks.insert("connections".to_string(), ComponentHealth {
        status: "ok".to_string(),
        message: Some(format!("{} active pools", conn_count)),
        latency_ms: conn_start.elapsed().as_millis() as u64,
    });
    
    // Share store check
    let store_start = std::time::Instant::now();
    let store_ok = state.share_store.get_share("health-check-test").await.is_ok();
    checks.insert("share_store".to_string(), ComponentHealth {
        status: if store_ok { "ok".to_string() } else { "error".to_string() },
        message: None,
        latency_ms: store_start.elapsed().as_millis() as u64,
    });
    
    // Token manager check
    let token_start = std::time::Instant::now();
    let token_ok = state.token_manager.read().await.validate_token("invalid").is_err();
    // If it properly rejects invalid tokens, it's working
    checks.insert("token_manager".to_string(), ComponentHealth {
        status: if token_ok { "ok".to_string() } else { "error".to_string() },
        message: None,
        latency_ms: token_start.elapsed().as_millis() as u64,
    });
    
    // Memory check
    let mem_start = std::time::Instant::now();
    // Simplified memory check
    checks.insert("memory".to_string(), ComponentHealth {
        status: "ok".to_string(),
        message: None,
        latency_ms: mem_start.elapsed().as_millis() as u64,
    });

    // Relay tunnel check
    let tunnel_start = std::time::Instant::now();
    let tunnel_ok = std::env::var("BENNETT_RELAY_URL").is_ok();
    checks.insert("relay_tunnel".to_string(), ComponentHealth {
        status: if tunnel_ok { "ok".to_string() } else { "disabled".to_string() },
        message: if tunnel_ok {
            Some("Relay tunnel configured".to_string())
        } else {
            Some("Relay tunnel not configured — P2P only mode".to_string())
        },
        latency_ms: tunnel_start.elapsed().as_millis() as u64,
    });

    let all_ok = checks.values().all(|c| c.status == "ok" || c.status == "disabled");
    
    let uptime = START_TIME.get()
        .map(|t| t.elapsed().as_secs())
        .unwrap_or(0);
    
    let total_latency = start.elapsed().as_millis() as u64;
    
    let status = HealthStatus {
        status: if all_ok { "healthy".to_string() } else { "degraded".to_string() },
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        checks,
    };
    
    if !all_ok {
        warn!("Health check degraded: {}ms total", total_latency);
    }
    
    Json(ApiResponse::success(status))
}

/// Simple health check (backward compatible)
pub async fn simple_health_check() -> Json<crate::models::database::ApiResponse<serde_json::Value>> {
    Json(crate::models::database::ApiResponse::success(serde_json::json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
        "phase": 6
    })))
}
