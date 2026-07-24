//! Bennett Relay Server
//! Public-facing TLS proxy for Bennett Studio database shares
//!
//! Architecture:
//! - TLS termination on 0.0.0.0:443
//! - Protocol detection (HTTP vs MySQL wire)
//! - Share ID extraction from URL path or MySQL username
//! - Route lookup in SQLite (engine's share store)
//! - Bidirectional TCP proxy to local engine
//! - Graceful shutdown on SIGTERM/SIGINT
//!
//! Future: P2P transport fallback via WebRTC/QUIC

mod api_key_registry;
mod api_v1;
mod config;
mod health;
mod metrics;
mod multiplexer;
mod rate_limit;
mod router;
mod server;
mod signaling;
mod transport;
mod tunnel_registry;
mod wire_frame;
mod wire_registry;

use axum::{
    extract::{
        Path, State, Json,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post, options},
    Router,
};
use base64::Engine;
use clap::Parser;
use futures_util::{sink::SinkExt, stream::StreamExt};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::signal;
use tracing::{debug, error, info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse configuration
    let config = config::RelayConfig::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(&config.log_level)
        .with_target(true)
        .init();

    // Install rustls crypto provider (required by rustls 0.23)
    let _ = tokio_rustls::rustls::crypto::aws_lc_rs::default_provider()
        .install_default();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        bind = %config.bind,
        "Bennett Relay Server starting"
    );

    // Resolve database path
    let db_path = config.resolve_db_path();

    // Initialize share router (reads engine's SQLite DB)
    let router = router::ShareRouter::new(
        &db_path,
        config.engine_http.port(),
        config.engine_mysql.port(),
    )
    .await
    .map_err(|e| {
        error!("Failed to initialize share router: {}", e);
        e
    })?;

    // Start background route refresh
    let _refresh_handle = router.start_refresh_task(config.route_refresh);

    // Gather ICE candidates if requested
    if config.gather_ice {
        match transport::ice::gather_ice_candidates().await {
            Ok(candidates) => {
                // Output base64-encoded ICE for easy embedding in URLs
                println!("{}", candidates.to_base64());
                return Ok(());
            }
            Err(e) => {
                eprintln!("Failed to gather ICE candidates: {}", e);
                std::process::exit(1);
            }
        }
    }

    // Create transport (pooled TCP, or P2P)
    let transport: Arc<dyn transport::Transport> = if config.enable_p2p {
        info!("P2P transport enabled");

        // Try P2P first, fall back to pooled TCP on any failure
        let p2p_result: anyhow::Result<Arc<dyn transport::Transport>> = async {
            if config.use_firebase {
                let firebase_url = config.firebase_url.clone()
                    .ok_or_else(|| anyhow::anyhow!("--firebase-url required for Firebase signaling"))?;
                let signaling = signaling::firebase::FirebaseSignaling::new(firebase_url);

                if config.p2p_mode == "engine" {
                    let room_code = config.share_code.clone()
                        .unwrap_or_else(signaling::firebase::generate_room_code);

                    let local_ice = transport::ice::gather_ice_candidates().await
                        .map_err(|e| anyhow::anyhow!("ICE gathering failed: {}", e))?;

                    match signaling.create_room(&room_code, &local_ice).await {
                        Ok(_) => info!(room = %room_code, "Firebase room created."),
                        Err(e) => warn!("Firebase signaling failed: {}. Continuing with P2P server.", e),
                    }

                    transport::TransportFactory::create_p2p_server(local_ice, Some(room_code), false).await
                        .map_err(|e| anyhow::anyhow!("P2P server init failed: {}", e))
                } else if config.p2p_mode == "client" {
                    let room_code = config.share_code.clone()
                        .ok_or_else(|| anyhow::anyhow!("--share-code required for Firebase client mode"))?;

                    let local_ice = transport::ice::gather_ice_candidates().await
                        .map_err(|e| anyhow::anyhow!("ICE gathering failed: {}", e))?;

                    let engine_ice = signaling.join_room(&room_code, &local_ice).await
                        .map_err(|e| anyhow::anyhow!("Failed to join room: {}", e))?;

                    transport::TransportFactory::create_p2p_client(engine_ice, Some(room_code)).await
                        .map_err(|e| anyhow::anyhow!("P2P client init failed: {}", e))
                } else {
                    Err(anyhow::anyhow!("--p2p-mode must be 'engine' or 'client'"))
                }
            } else if let Some(remote_ice_b64) = &config.remote_ice {
                let remote_ice = transport::ice::IceCandidates::from_base64(remote_ice_b64)
                    .map_err(|e| anyhow::anyhow!("Invalid remote ICE: {}", e))?;
                transport::TransportFactory::create_p2p_client(remote_ice, config.share_code.clone()).await
                    .map_err(|e| anyhow::anyhow!("P2P client init failed: {}", e))
            } else {
                let local_ice = transport::ice::gather_ice_candidates().await
                    .map_err(|e| anyhow::anyhow!("ICE gathering failed: {}", e))?;
                info!("P2P server ICE: {}", serde_json::to_string_pretty(&local_ice).unwrap());
                transport::TransportFactory::create_p2p_server(local_ice, config.share_code.clone(), false).await
                    .map_err(|e| anyhow::anyhow!("P2P server init failed: {}", e))
            }
        }.await;

        match p2p_result {
            Ok(transport) => {
                info!("P2P transport initialized successfully");
                transport
            }
            Err(e) => {
                return Err(anyhow::anyhow!("P2P transport failed: {}. Use --enable-p2p=false for TCP mode.", e));
            }
        }
    } else {
        info!("Pooled TCP transport active (connection pooling + splice)");
        transport::TransportFactory::create_pooled_tcp(
            config.engine_http,
            config.engine_mysql,
            config.max_conn_per_share,
        )
    };

    // In P2P server mode, print the base64 ICE for client connection
    if config.enable_p2p && config.p2p_mode == "engine" {
        if let Some(p2p) = transport.as_any().downcast_ref::<transport::p2p::P2pTransport>() {
            let ice_b64 = p2p.local_ice().to_base64();
            info!("P2P server ICE (base64): {}", ice_b64);
            println!("=== SHARE THIS ICE WITH CLIENTS ===");
            println!("{}", ice_b64);
            println!("===================================");
        }
    }

    // In HTTP mode, the proxy API is served by RelayServer::run_http_mode()
    // In TLS mode, we spawn a separate proxy API for local development
    let _api_handle = if !config.http_mode {
        let router_clone = router.clone();
        let _transport_clone = transport.clone();
        let bind_addr = config.proxy_api_bind.to_string();

        println!("DEBUG MAIN: bind_addr={}", bind_addr);
        info!(addr = %bind_addr, "Starting HTTP proxy API for external website access");

        let rate_rps = config.api_v1_rate_rps;
        let rate_burst = config.api_v1_rate_burst;
        Some(tokio::spawn(async move {
            println!("DEBUG PROXY: Starting HTTP proxy API task on {}", bind_addr);
            if let Err(e) = start_http_proxy_api(bind_addr, router_clone, _transport_clone, rate_rps, rate_burst).await {
                error!("HTTP proxy API error: {}", e);
            }
            println!("DEBUG PROXY: HTTP proxy API task ended");
        }))
    } else {
        info!("HTTP mode: proxy API served on main bind, skipping separate proxy API");
        None
    };

    // Yield only if we spawned a separate task
    if _api_handle.is_some() {
        tokio::task::yield_now().await;
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // Start health monitor
    let _health_handle = health::HealthMonitor::start(
        router.clone(),
        transport.clone(),
        config.health_interval,
    );

    // Start metrics HTTP endpoint (separate from TLS relay)
    let _metrics_handle = metrics::start_metrics_server(config.bind.port() + 1000).await;

    // Shutdown channel
    let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

    // Spawn signal handler
    let shutdown_tx_clone = shutdown_tx.clone();
    tokio::spawn(async move {
        let mut sigterm = signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to register SIGTERM handler");
        let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
            .expect("Failed to register SIGINT handler");

        tokio::select! {
            _ = sigterm.recv() => {
                info!("Received SIGTERM, initiating graceful shutdown");
            }
            _ = sigint.recv() => {
                info!("Received SIGINT, initiating graceful shutdown");
            }
        }

        let _ = shutdown_tx_clone.send(true);
    });

    // Also handle Ctrl+C for Windows/non-Unix
    #[cfg(not(unix))]
    {
        let shutdown_tx_clone = shutdown_tx.clone();
        tokio::spawn(async move {
            if let Err(e) = tokio::signal::ctrl_c().await {
                warn!("Failed to listen for ctrl-c: {}", e);
                return;
            }
            info!("Received Ctrl+C, initiating graceful shutdown");
            let _ = shutdown_tx_clone.send(true);
        });
    }

    // Initialize and run relay server
    let relay = server::RelayServer::new(config, router, transport).await?;

    info!("Relay server ready — waiting for connections");

    // Run server with shutdown support
    // The api_handle must stay alive across this await so the task isn't cancelled
    relay.run(shutdown_rx).await?;

    info!("Relay server shutdown complete");

    // Explicitly keep the handle alive until shutdown (prevents premature drop)
    if let Some(handle) = _api_handle {
        drop(handle);
    }

    Ok(())
}

// ============================================================================
// PHASE 5: WebRTC Signaling Handlers
// Browsers initiate P2P via POST /api/share/:code/webrtc/offer
// Relay responds with SDP answer and manages the bridge
// ============================================================================

#[derive(Debug, Clone, serde::Deserialize)]
struct WebRtcOfferRequest {
    pub sdp: String,
    pub ice_candidates: Vec<serde_json::Value>,
    pub token: String,
}

#[derive(Debug, Clone, serde::Serialize)]
struct WebRtcOfferResponse {
    pub success: bool,
    pub answer_sdp: Option<String>,
    pub answer_ice: Vec<serde_json::Value>,
    pub error: Option<String>,
}

/// Handle WebRTC offer from browser
/// Returns SDP answer for browser to complete P2P
pub async fn webrtc_offer_handler(
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
    req_headers: axum::http::HeaderMap,
    Json(_req): Json<WebRtcOfferRequest>,
) -> impl IntoResponse {
    let origin = req_headers.get(axum::http::header::ORIGIN).and_then(|v| v.to_str().ok());
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in router::cors_headers(origin) {
        headers.insert(k, v.parse().unwrap());
    }

    // Validate share is active
    if !state.router.is_active(&code).await {
        return (
            StatusCode::NOT_FOUND,
            headers,
            axum::Json(WebRtcOfferResponse {
                success: false,
                answer_sdp: None,
                answer_ice: vec![],
                error: Some("Share not found or expired".to_string()),
            }),
        );
    }

    // Validate token
    // TODO: integrate with token validation

    info!(share = %code, "WebRTC offer received from browser");

    // PHASE 5: If WebRTC bridge is available, process offer
    // For now, return a placeholder indicating relay mode
    // Full WebRTC bridge requires the webrtc feature flag
    
    (
        StatusCode::OK,
        headers,
        axum::Json(WebRtcOfferResponse {
            success: true,
            answer_sdp: None, // Would contain SDP answer if bridge active
            answer_ice: vec![],
            error: Some("WebRTC P2P not yet active — use WebSocket relay instead".to_string()),
        }),
    )
}

#[derive(Debug, Clone, serde::Deserialize)]
struct WebRtcIceRequest {
    pub candidate: serde_json::Value,
    pub token: String,
}

/// Handle ICE candidate trickle from browser
pub async fn webrtc_ice_handler(
    Path(code): Path<String>,
    State(_state): State<ProxyApiState>,
    req_headers: axum::http::HeaderMap,
    Json(_req): Json<WebRtcIceRequest>,
) -> impl IntoResponse {
    let origin = req_headers.get(axum::http::header::ORIGIN).and_then(|v| v.to_str().ok());
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in router::cors_headers(origin) {
        headers.insert(k, v.parse().unwrap());
    }

    // Store ICE candidate for ongoing P2P negotiation
    // Implementation depends on active WebRTC bridge
    
    (StatusCode::OK, headers, axum::Json(serde_json::json!({ "received": true })))
}

/// HTTP Proxy API for external websites
/// Allows any website (Vercel, etc.) to query the shared database
/// through the P2P tunnel via simple HTTP requests
async fn start_http_proxy_api(
    bind_addr: String,
    router: Arc<router::ShareRouter>,
    _transport: Arc<dyn transport::Transport>,
    api_v1_rate_rps: u32,
    api_v1_rate_burst: u32,
) -> anyhow::Result<()> {
    // Use eprintln to bypass any tracing/log capture and guarantee visibility
    eprintln!("DEBUG PROXY: bind_addr={}", bind_addr);
// Initialize shared tunnel registry
    let tunnel_registry = crate::tunnel_registry::TunnelRegistry::new();

    // Inject tunnel registry into router
    let router = router.with_tunnel_registry(tunnel_registry.clone());

    // Initialize API key routing table (key_hash -> host_id)
    let api_key_registry = crate::api_key_registry::ApiKeyRegistry::new();

    // Initialize /api/v1 rate limiter — per-key throttling is the primary
    // defense here since durable keys have no expiry to bound abuse
    let rate_limiter = Arc::new(crate::rate_limit::ApiV1RateLimiter::new(api_v1_rate_rps, api_v1_rate_burst));

    // Initialize wire-protocol (MySQL/Postgres) tunneled stream registry (Phase 2)
    let wire_stream_registry = crate::wire_registry::WireStreamRegistry::new();

    let app_state = ProxyApiState {
        router,
        tunnel_registry: tunnel_registry.clone(),
        api_key_registry,
        rate_limiter,
        wire_stream_registry,
    };

    // CORS middleware for external web app access
    use tower_http::cors::{CorsLayer, Any};
    use axum::http::{HeaderValue, Method};

    let cors = CorsLayer::new()
        .allow_origin([
            "https://share-bennett-studio.vercel.app".parse::<HeaderValue>().unwrap(),
            "https://app-bennett-studio.vercel.app".parse::<HeaderValue>().unwrap(),
            "http://localhost:5173".parse::<HeaderValue>().unwrap(),
            "http://localhost:5174".parse::<HeaderValue>().unwrap(),
            "http://localhost:3000".parse::<HeaderValue>().unwrap(),
            "http://localhost:3001".parse::<HeaderValue>().unwrap(),
            "tauri://localhost".parse::<HeaderValue>().unwrap(),
        ])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
            axum::http::header::HeaderName::from_static("x-share-code"),
            axum::http::header::HeaderName::from_static("x-share-token"),
            axum::http::header::HeaderName::from_static("x-requested-with"),
        ])
        .allow_credentials(true);

    let app_v1_cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ]);

    let app_v1 = Router::new()
        // Stable public API gateway — durable Bearer bnt_live_ keys, no expiry,
        // meant to be hardcoded as a base URL by external apps, unlike the
        // ephemeral /db/:code share links.
        .route("/api/v1/health", get(api_v1::api_v1_health))
        .route("/api/v1/query", post(api_v1::api_v1_query))
        .route("/api/v1/schema", get(api_v1::api_v1_schema))
        .layer(app_v1_cors)
        .with_state(app_state.clone());

    let app = Router::new()
        // Health check
        .route("/health", get(proxy_health))
        .route("/api/health", get(proxy_health))
        // CORS preflight
        .route("/api/share/:code/query", options(cors_preflight))
        .route("/api/share/:code/schema", options(cors_preflight))
        .route("/ws/share/:code", options(cors_preflight))
        // PHASE 5: WebRTC signaling endpoint for browser P2P
        .route("/api/share/:code/webrtc/offer", post(webrtc_offer_handler))
        .route("/api/share/:code/webrtc/ice", post(webrtc_ice_handler))
        // Query execution
        .route("/api/share/:code/query", post(proxy_query))
        // Schema fetch
        .route("/api/share/:code/schema", get(proxy_schema))
        // Share info
        .route("/api/share/:code", get(proxy_share_info))
        // Share validation (guest)
        .route("/api/share/:code/validate", post(proxy_validate_share))
        // WebSocket streaming for real-time queries
        .route("/ws/share/:code", get(ws_proxy_handler))
        // PHASE A: Engine tunnel WebSocket — engines connect here to register routes
        .route("/ws/tunnel/:host_id", get(tunnel_ws_handler))
        .layer(cors)
        .with_state(app_state)
        .merge(app_v1);

    let addr: SocketAddr = bind_addr.parse()?;
    info!("HTTP proxy API listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Clone)]
pub struct ProxyApiState {
    router: Arc<router::ShareRouter>,
    tunnel_registry: Arc<crate::tunnel_registry::TunnelRegistry>,
    api_key_registry: Arc<crate::api_key_registry::ApiKeyRegistry>,
    rate_limiter: Arc<crate::rate_limit::ApiV1RateLimiter>,
    wire_stream_registry: Arc<crate::wire_registry::WireStreamRegistry>,
}

#[derive(Debug, serde::Deserialize)]
struct SchemaQueryParams {
    token: String,
}

/// CORS preflight response
pub async fn cors_preflight(req_headers: axum::http::HeaderMap) -> impl IntoResponse {
    let origin = req_headers.get(axum::http::header::ORIGIN).and_then(|v| v.to_str().ok());
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in router::cors_headers(origin) {
        headers.insert(k, v.parse().unwrap());
    }
    (StatusCode::NO_CONTENT, headers)
}

/// Health check for proxy API
pub async fn proxy_health() -> impl IntoResponse {
    info!("Proxy health check received");
    let body = serde_json::json!({ "status": "ok", "service": "bennett-proxy" });
    (StatusCode::OK, axum::Json(body))
}

/// Execute a query through the proxy
/// External websites POST here with { sql, token }
/// Forwards to engine's POST /api/shares/:code/query endpoint
pub async fn proxy_query(
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
    req_headers: axum::http::HeaderMap,
    Json(req): Json<router::ProxyQueryRequest>,
) -> impl IntoResponse {
    let origin = req_headers.get(axum::http::header::ORIGIN).and_then(|v| v.to_str().ok());
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in router::cors_headers(origin) {
        headers.insert(k, v.parse().unwrap());
    }

    // Check if share is active
    if !state.router.is_active(&code).await {
        return (
            StatusCode::NOT_FOUND,
            headers,
            axum::Json(serde_json::json!({
                "success": false,
                "error": "Share not found or expired"
            })),
        );
    }

    info!(share = %code, sql_len = %req.sql.len(), "Proxy query received — forwarding to engine");

    // Build request body for engine
    let engine_body = serde_json::json!({
        "sql": req.sql,
        "limit": req.limit,
        "offset": req.offset,
    });

    // Forward to engine via TCP (relay and engine run on same host)
    match state.router.forward_to_engine(
        &code,
        "POST",
        &format!("/api/shares/{}/query", code),
        Some(engine_body.to_string().into_bytes()),
        &req.token,
    ).await {
        Ok(response_bytes) => {
            // Parse engine's HTTP response to extract JSON body
            let response_str = String::from_utf8_lossy(&response_bytes);
            
            // Find the JSON body (after \r\n\r\n)
            if let Some(body_start) = response_str.find("\r\n\r\n") {
                let body = &response_str[body_start + 4..];
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                    return (StatusCode::OK, headers, axum::Json(json));
                }
            }
            
            // Fallback: return raw response
            (
                StatusCode::OK,
                headers,
                axum::Json(serde_json::json!({
                    "success": true,
                    "raw_response": response_str.to_string(),
                })),
            )
        }
        Err(e) => {
            warn!(share = %code, error = %e, "Engine forwarding failed");
            (
                StatusCode::BAD_GATEWAY,
                headers,
                axum::Json(serde_json::json!({
                    "success": false,
                    "error": format!("Engine forwarding failed: {}", e)
                })),
            )
        }
    }
}

/// Get schema through the proxy
/// Forwards to engine's GET /api/shares/:code/schema endpoint
pub async fn proxy_schema(
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
    req_headers: axum::http::HeaderMap,
    axum::extract::Query(params): axum::extract::Query<SchemaQueryParams>,
) -> impl IntoResponse {
    let token = params.token;
    eprintln!("DEBUG SCHEMA PROXY: code={}, token_len={}, token_prefix={}",
        code, token.len(), &token[..token.len().min(20)]);
    eprintln!("DEBUG SCHEMA PROXY: code={}, token_len={}, token_empty={}, token_prefix={}",
        code, token.len(), token.is_empty(), &token[..token.len().min(20)]);
    let origin = req_headers.get(axum::http::header::ORIGIN).and_then(|v| v.to_str().ok());
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in router::cors_headers(origin) {
        headers.insert(k, v.parse().unwrap());
    }

    if !state.router.is_active(&code).await {
        return (
            StatusCode::NOT_FOUND,
            headers,
            axum::Json(serde_json::json!({
                "success": false,
                "error": "Share not found or expired"
            })),
        );
    }

    info!(share = %code, "Proxy schema request received — forwarding to engine");

    // Forward to engine via TCP
    match state.router.forward_to_engine(
        &code,
        "GET",
        &format!("/api/shares/{}/schema", code),
        None,
        &token,
    ).await {
        Ok(response_bytes) => {
            let response_str = String::from_utf8_lossy(&response_bytes);
            
            if let Some(body_start) = response_str.find("\r\n\r\n") {
                let body = &response_str[body_start + 4..];
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                    return (StatusCode::OK, headers, axum::Json(json));
                }
            }
            
            (
                StatusCode::OK,
                headers,
                axum::Json(serde_json::json!({
                    "success": true,
                    "raw_response": response_str.to_string(),
                })),
            )
        }
        Err(e) => {
            warn!(share = %code, error = %e, "Engine schema forwarding failed");
            (
                StatusCode::BAD_GATEWAY,
                headers,
                axum::Json(serde_json::json!({
                    "success": false,
                    "error": format!("Engine forwarding failed: {}", e)
                })),
            )
        }
    }
}

/// Validate a share through the proxy
/// Checks if share is active in relay cache/tunnel registry
/// Does NOT forward to engine — token validation happens at query time
pub async fn proxy_validate_share(
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
    req_headers: axum::http::HeaderMap,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    let origin = req_headers.get(axum::http::header::ORIGIN).and_then(|v| v.to_str().ok());
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in router::cors_headers(origin) {
        headers.insert(k, v.parse().unwrap());
    }

    if !state.router.is_active(&code).await {
        return (
            StatusCode::NOT_FOUND,
            headers,
            axum::Json(serde_json::json!({
                "success": false,
                "error": "Share not found or expired"
            })),
        );
    }

    info!(share = %code, "Proxy validate request received — share is active");

    // Extract token from request to decode JWT and return share metadata
    let token = req.get("token").and_then(|t| t.as_str()).unwrap_or("");
    let (db_id, permission, tables, expires_at) = if !token.is_empty() {
        // Decode JWT payload to extract metadata
        if let Some(payload) = decode_jwt_payload(token) {
            (
                payload.get("db_id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string(),
                payload.get("perm").and_then(|v| v.as_str()).unwrap_or("ro").to_string(),
                payload.get("tables").and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_else(|| vec!["*".to_string()]),
                payload.get("exp").and_then(|v| v.as_i64()).unwrap_or(0),
            )
        } else {
            ("unknown".to_string(), "ro".to_string(), vec!["*".to_string()], 0)
        }
    } else {
        ("unknown".to_string(), "ro".to_string(), vec!["*".to_string()], 0)
    };

    (
        StatusCode::OK,
        headers,
        axum::Json(serde_json::json!({
            "success": true,
            "data": {
                "valid": true,
                "code": code,
                "db_id": db_id,
                "permission": permission,
                "tables": tables,
                "expires_at": expires_at,
                "host_online": true
            }
        })),
    )
}

/// Decode JWT payload without verification (for metadata extraction)
fn decode_jwt_payload(token: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let base64 = parts[1].replace('-', "+").replace('_', "/");
    let pad_len = (4 - (base64.len() % 4)) % 4;
    let padded = base64 + &"=".repeat(pad_len);
    let decoded = base64::engine::general_purpose::STANDARD.decode(&padded).ok()?;
    serde_json::from_slice(&decoded).ok()
}

/// Get public share info
pub async fn proxy_share_info(
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
    req_headers: axum::http::HeaderMap,
) -> impl IntoResponse {
    let origin = req_headers.get(axum::http::header::ORIGIN).and_then(|v| v.to_str().ok());
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in router::cors_headers(origin) {
        headers.insert(k, v.parse().unwrap());
    }

    if !state.router.is_active(&code).await {
        return (
            StatusCode::NOT_FOUND,
            headers,
            axum::Json(serde_json::json!({
                "success": false,
                "error": "Share not found or expired"
            })),
        );
    }

    (
        StatusCode::OK,
        headers,
        axum::Json(serde_json::json!({
            "success": true,
            "code": code,
            "status": "active",
            "message": "Share is active — connect with BennettClient"
        })),
    )
}

// ============================================================================
// WebSocket Streaming Proxy
// ============================================================================

/// WebSocket request messages from client
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsProxyRequest {
    /// Execute a query
    Query { sql: String, limit: Option<i64>, request_id: Option<String> },
    /// Subscribe to schema changes
    SubscribeSchema,
    /// Ping to keep connection alive
    Ping,
    /// Acknowledge receipt (for flow control)
    Ack { message_id: u64 },
}

/// WebSocket response messages to client
#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum WsProxyResponse {
    /// Query result
    QueryResult {
        columns: Vec<String>,
        rows: Vec<Vec<serde_json::Value>>,
        row_count: usize,
        execution_time_ms: u64,
        request_id: Option<String>,
        message_id: u64,
    },
    /// Query error
    QueryError {
        error: String,
        request_id: Option<String>,
        message_id: u64,
    },
    /// Schema update
    SchemaUpdate {
        tables: Vec<serde_json::Value>,
        database_name: String,
        database_type: String,
        message_id: u64,
    },
    /// Connection status
    Status {
        connected: bool,
        share_code: String,
        message: String,
        message_id: u64,
    },
    /// Pong response
    Pong,
    /// Server error
    Error { message: String, message_id: u64 },
}

// ============================================================================
// PHASE A: Engine Tunnel WebSocket Handler
// Engines connect here to register their shares and receive forwarded queries
// ============================================================================

#[derive(Clone)]
struct TunnelState {
    router: Arc<router::ShareRouter>,
    // Map of host_id -> WebSocket sender for forwarding queries back
    tunnels: Arc<tokio::sync::RwLock<std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<TunnelEngineMessage>>>>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TunnelEngineMessage {
    QueryRequest {
        request_id: String,
        share_code: String,
        token: String,
        sql: String,
        limit: Option<i32>,
        offset: Option<i32>,
    },
    SchemaRequest {
        request_id: String,
        share_code: String,
        token: String,
    },
    ApiKeyQueryRequest {
        request_id: String,
        key_hash: String,
        sql: String,
        limit: Option<i32>,
        offset: Option<i32>,
    },
    ApiKeySchemaRequest {
        request_id: String,
        key_hash: String,
    },
    WireStreamOpen {
        stream_id: String,
        wire_username: String,
        wire_password_hash: String,
    },
    WireStreamClose {
        stream_id: String,
    },
    Ping,
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum TunnelEngineResponse {
    QueryResponse {
        request_id: String,
        result: serde_json::Value,
    },
    SchemaResponse {
        request_id: String,
        schema: serde_json::Value,
    },
    Register {
        host_id: String,
        version: String,
        capabilities: Vec<String>,
    },
    ShareCreated {
        code: String,
        db_id: String,
        permission: String,
        expires_at: i64,
    },
    ShareRevoked {
        code: String,
    },
    ApiKeyRegistered {
        key_hash: String,
        db_id: String,
        permission: String,
    },
    ApiKeyRevoked {
        key_hash: String,
    },
    ApiKeyQueryResponse {
        request_id: String,
        result: serde_json::Value,
    },
    ApiKeySchemaResponse {
        request_id: String,
        schema: serde_json::Value,
    },
    /// Engine confirms a tunneled wire-protocol stream opened successfully
    WireStreamOpened {
        stream_id: String,
    },
    /// Engine reports a tunneled wire-protocol stream failed to open
    WireStreamError {
        stream_id: String,
        message: String,
    },
    Pong,
}

/// WebSocket handler for engine tunnels
pub async fn tunnel_ws_handler(
    ws: WebSocketUpgrade,
    Path(host_id): Path<String>,
    State(state): State<ProxyApiState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_tunnel_ws(socket, host_id, state))
}

async fn handle_tunnel_ws(
    socket: WebSocket,
    host_id: String,
    state: ProxyApiState,
) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<TunnelEngineMessage>();

    info!("Engine tunnel connected: host_id={}", host_id);
    // Mark host as online (restores any previously offline routes)
    state.router.mark_host_online(&host_id).await;

    // Register tunnel in shared registry
    // Convert our TunnelEngineMessage sender to TunnelMessageToEngine sender
    // They have the same shape, so we use a wrapper channel
    let (wrap_tx, mut wrap_rx) = tokio::sync::mpsc::unbounded_channel::<crate::tunnel_registry::TunnelMessageToEngine>();
    
    // Bridge: convert TunnelMessageToEngine -> TunnelEngineMessage and send via original tx
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        while let Some(msg) = wrap_rx.recv().await {
            let engine_msg = match msg {
                crate::tunnel_registry::TunnelMessageToEngine::QueryRequest { request_id, share_code, token, sql, limit, offset } => {
                    TunnelEngineMessage::QueryRequest { request_id, share_code, token, sql, limit, offset }
                }
                crate::tunnel_registry::TunnelMessageToEngine::SchemaRequest { request_id, share_code, token } => {
                    TunnelEngineMessage::SchemaRequest { request_id, share_code, token }
                }
                crate::tunnel_registry::TunnelMessageToEngine::ApiKeyQueryRequest { request_id, key_hash, sql, limit, offset } => {
                    TunnelEngineMessage::ApiKeyQueryRequest { request_id, key_hash, sql, limit, offset }
                }
                crate::tunnel_registry::TunnelMessageToEngine::ApiKeySchemaRequest { request_id, key_hash } => {
                    TunnelEngineMessage::ApiKeySchemaRequest { request_id, key_hash }
                }
                crate::tunnel_registry::TunnelMessageToEngine::WireStreamOpen { stream_id, wire_username, wire_password_hash } => {
                    TunnelEngineMessage::WireStreamOpen { stream_id, wire_username, wire_password_hash }
                }
                crate::tunnel_registry::TunnelMessageToEngine::WireStreamClose { stream_id } => {
                    TunnelEngineMessage::WireStreamClose { stream_id }
                }
                crate::tunnel_registry::TunnelMessageToEngine::Ping => TunnelEngineMessage::Ping,
            };
            let _ = tx_clone.send(engine_msg);
        }
    });

    state.tunnel_registry.register_tunnel(host_id.clone(), wrap_tx).await;
    info!("Registered tunnel in registry for host: {}", host_id);

    // Phase 2: dedicated channel for raw binary wire-protocol frames
    // (MySQL/Postgres bytes), kept separate from the JSON control channel
    let (wire_bin_tx, mut wire_bin_rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();
    state.tunnel_registry.register_wire_tunnel(host_id.clone(), wire_bin_tx).await;

    // Send welcome ping
    let _ = sender.send(Message::Text(
        serde_json::to_string(&TunnelEngineMessage::Ping).unwrap()
    )).await;

    // Main loop: handle messages from engine and from relay
    loop {
        tokio::select! {
            // Messages from engine
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(resp) = serde_json::from_str::<TunnelEngineResponse>(&text) {
                            match resp {
                                TunnelEngineResponse::Register { host_id, version, capabilities } => {
                                    info!("Engine registered: {} (v{}, caps: {:?})", host_id, version, capabilities);
                                }
                                TunnelEngineResponse::ShareCreated { code, db_id, permission: _, expires_at } => {
                                    info!("Engine reported share created: {}", code);
                                    // Add remote route so relay can forward to this engine
                                    let route = router::ShareRoute {
                                        share_id: code.clone(),
                                        db_id,
                                        protocol: crate::transport::ProtocolType::ConnectRpc,
                                        engine_port: 0, // Tunnel doesn't use port
                                        expires_at: chrono::DateTime::from_timestamp(expires_at, 0)
                                            .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::hours(24)),
                                        revoked: false,
                                        host_id: Some(host_id.clone()), // Track which host owns this share
                                    };
                                    state.router.add_remote_route(route).await;
                                }
                                TunnelEngineResponse::ShareRevoked { code } => {
                                    info!("Engine reported share revoked: {}", code);
                                    state.router.remove_remote_route(&code).await;
                                }
                                TunnelEngineResponse::ApiKeyRegistered { key_hash, .. } => {
                                    info!("Engine registered API key for host: {}", host_id);
                                    state.api_key_registry.register(key_hash, host_id.clone());
                                }
                                TunnelEngineResponse::ApiKeyRevoked { key_hash } => {
                                    info!("Engine revoked API key");
                                    state.api_key_registry.revoke(&key_hash);
                                    state.rate_limiter.remove_key(&key_hash).await;
                                }
                                TunnelEngineResponse::ApiKeyQueryResponse { request_id, result } => {
                                    debug!("Engine API key query response: {}", request_id);
                                    state.tunnel_registry.complete_request(&request_id, result).await;
                                }
                                TunnelEngineResponse::ApiKeySchemaResponse { request_id, schema } => {
                                    debug!("Engine API key schema response: {}", request_id);
                                    state.tunnel_registry.complete_request(&request_id, schema).await;
                                }
                                TunnelEngineResponse::WireStreamOpened { stream_id } => {
                                    debug!("Engine confirmed wire stream opened: {}", stream_id);
                                    state.wire_stream_registry.complete_open(&stream_id, Ok(()));
                                }
                                TunnelEngineResponse::WireStreamError { stream_id, message } => {
                                    warn!("Engine reported wire stream error for {}: {}", stream_id, message);
                                    state.wire_stream_registry.complete_open(&stream_id, Err(message));
                                }
                                TunnelEngineResponse::QueryResponse { request_id, result } => {
                                    debug!("Engine query response: {}", request_id);
                                    state.tunnel_registry.complete_request(&request_id, result).await;
                                }
                                TunnelEngineResponse::SchemaResponse { request_id, schema } => {
                                    debug!("Engine schema response: {}", request_id);
                                    state.tunnel_registry.complete_request(&request_id, schema).await;
                                }
                                TunnelEngineResponse::Pong => {
                                    debug!("Engine tunnel pong received");
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Binary(data))) => {
                        if let Some((msg_type, stream_id, payload)) = crate::wire_frame::decode_wire_frame(&data) {
                            if msg_type == crate::wire_frame::WIRE_FRAME_TYPE_DATA {
                                if !state.wire_stream_registry.route_to_client(stream_id, payload.to_vec()) {
                                    debug!("Wire frame for unknown/closed client stream {}", stream_id);
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        info!("Engine tunnel disconnected: host_id={}", host_id);
                        break;
                    }
                    Some(Err(e)) => {
                        warn!("Engine tunnel error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // Messages to forward to engine
            Some(msg) = rx.recv() => {
                let text = serde_json::to_string(&msg).unwrap();
                if let Err(e) = sender.send(Message::Text(text)).await {
                    warn!("Failed to send to engine tunnel: {}", e);
                    break;
                }
            }

            // Binary wire-protocol frames to forward to engine (Phase 2)
            Some(frame) = wire_bin_rx.recv() => {
                if let Err(e) = sender.send(Message::Binary(frame)).await {
                    warn!("Failed to send wire binary frame to engine: {}", e);
                    break;
                }
            }

            // Keepalive
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                let _ = sender.send(Message::Text(
                    serde_json::to_string(&TunnelEngineMessage::Ping).unwrap()
                )).await;
            }
        }
    }

    info!("Engine tunnel closed: host_id={}", host_id);
    // Cleanup tunnel registry
    state.tunnel_registry.unregister_tunnel(&host_id).await;
    state.tunnel_registry.unregister_wire_tunnel(&host_id).await;
    // Cleanup ALL routes from this host
    state.router.remove_all_host_routes(&host_id).await;
    // Cleanup ALL API keys registered by this host
    state.api_key_registry.remove_all_host_keys(&host_id);
}

/// WebSocket upgrade handler
pub async fn ws_proxy_handler(
    ws: WebSocketUpgrade,
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_proxy(socket, code, state))
}

/// Handle WebSocket connection for share streaming
pub async fn handle_ws_proxy(
    socket: WebSocket,
    code: String,
    state: ProxyApiState,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut message_id: u64 = 0;

    info!(share = %code, "WebSocket proxy connection established");

    // Send initial status
    message_id += 1;
    let connected = state.router.is_active(&code).await;
    let status_msg = WsProxyResponse::Status {
        connected,
        share_code: code.clone(),
        message: "Connected to Bennett relay proxy".to_string(),
        message_id,
    };
    let _ = sender.send(Message::Text(
        serde_json::to_string(&status_msg).unwrap()
    )).await;

    // If share not active, close connection
    if !state.router.is_active(&code).await {
        message_id += 1;
        let _ = sender.send(Message::Text(
            serde_json::to_string(&WsProxyResponse::Error {
                message: "Share not found or expired".to_string(),
                message_id,
            }).unwrap()
        )).await;
        let _ = sender.close().await;
        return;
    }

    // Main message loop
    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<WsProxyRequest>(&text) {
                            Ok(WsProxyRequest::Ping) => {
                                let _ = sender.send(Message::Text(
                                    serde_json::to_string(&WsProxyResponse::Pong).unwrap()
                                )).await;
                            }
                            Ok(WsProxyRequest::Query { sql, limit, request_id }) => {
                                info!(share = %code, sql_len = %sql.len(), "WebSocket query received");

                                // Validate SQL (basic)
                                if sql.trim().is_empty() {
                                    message_id += 1;
                                    let _ = sender.send(Message::Text(
                                        serde_json::to_string(&WsProxyResponse::QueryError {
                                            error: "Empty SQL query".to_string(),
                                            request_id: request_id.clone(),
                                            message_id,
                                        }).unwrap()
                                    )).await;
                                    continue;
                                }

                                // Forward query to engine via HTTP through the router
                                message_id += 1;
                                let start = std::time::Instant::now();

                                let engine_body = serde_json::json!({
                                    "sql": sql,
                                    "limit": limit.unwrap_or(1000),
                                });

                                match state.router.forward_to_engine(
                                    &code,
                                    "POST",
                                    &format!("/api/shares/{}/query", code),
                                    Some(engine_body.to_string().into_bytes()),
                                    "", // Token handling TBD: extract from connection state
                                ).await {
                                    Ok(response_bytes) => {
                                        let response_str = String::from_utf8_lossy(&response_bytes);
                                        
                                        // Try to parse engine response and extract query results
                                        if let Some(body_start) = response_str.find("\r\n\r\n") {
                                            let body = &response_str[body_start + 4..];
                                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                                                // Extract data from engine's ApiResponse wrapper
                                                let data = json.get("data").unwrap_or(&json);
                                                let columns = data.get("columns")
                                                    .and_then(|c| c.as_array())
                                                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                                                    .unwrap_or_else(|| vec!["result".to_string()]);
                                                let rows = data.get("rows")
                                                    .and_then(|r| r.as_array())
                                                    .map(|arr| arr.iter().map(|v| vec![v.clone()]).collect())
                                                    .unwrap_or_default();

                                                let response = WsProxyResponse::QueryResult {
                                                    columns,
                                                    rows,
                                                    row_count: data.get("row_count").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                                                    execution_time_ms: start.elapsed().as_millis() as u64,
                                                    request_id,
                                                    message_id,
                                                };
                                                let _ = sender.send(Message::Text(
                                                    serde_json::to_string(&response).unwrap()
                                                )).await;
                                            } else {
                                                let response = WsProxyResponse::QueryResult {
                                                    columns: vec!["raw".to_string()],
                                                    rows: vec![vec![serde_json::json!(body)]],
                                                    row_count: 1,
                                                    execution_time_ms: start.elapsed().as_millis() as u64,
                                                    request_id,
                                                    message_id,
                                                };
                                                let _ = sender.send(Message::Text(
                                                    serde_json::to_string(&response).unwrap()
                                                )).await;
                                            }
                                        } else {
                                            let response = WsProxyResponse::QueryError {
                                                error: "Invalid engine response".to_string(),
                                                request_id,
                                                message_id,
                                            };
                                            let _ = sender.send(Message::Text(
                                                serde_json::to_string(&response).unwrap()
                                            )).await;
                                        }
                                    }
                                    Err(e) => {
                                        let response = WsProxyResponse::QueryError {
                                            error: format!("Engine forwarding failed: {}", e),
                                            request_id,
                                            message_id,
                                        };
                                        let _ = sender.send(Message::Text(
                                            serde_json::to_string(&response).unwrap()
                                        )).await;
                                    }
                                }
                            }
                            Ok(WsProxyRequest::SubscribeSchema) => {
                                info!(share = %code, "Schema subscription requested");

                                // TODO: Fetch schema through P2P tunnel
                                message_id += 1;
                                let response = WsProxyResponse::SchemaUpdate {
                                    tables: vec![],
                                    database_name: "Remote Database".to_string(),
                                    database_type: "unknown".to_string(),
                                    message_id,
                                };
                                let _ = sender.send(Message::Text(
                                    serde_json::to_string(&response).unwrap()
                                )).await;
                            }
                            Ok(WsProxyRequest::Ack { message_id: ack_id }) => {
                                debug!("Client acked message {}", ack_id);
                            }
                            Err(e) => {
                                message_id += 1;
                                let _ = sender.send(Message::Text(
                                    serde_json::to_string(&WsProxyResponse::Error {
                                        message: format!("Invalid message: {}", e),
                                        message_id,
                                    }).unwrap()
                                )).await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        info!(share = %code, "WebSocket client disconnected");
                        break;
                    }
                    Some(Err(e)) => {
                        warn!(share = %code, error = %e, "WebSocket error");
                        break;
                    }
                    _ => {}
                }
            }

            // Periodic keepalive
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(30)) => {
                message_id += 1;
                let connected = state.router.is_active(&code).await;
                let _ = sender.send(Message::Text(
                    serde_json::to_string(&WsProxyResponse::Status {
                        connected,
                        share_code: code.clone(),
                        message: "Keepalive".to_string(),
                        message_id,
                    }).unwrap()
                )).await;
            }
        }
    }

    info!(share = %code, "WebSocket proxy connection closed");
}
