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

mod config;
mod health;
mod metrics;
mod multiplexer;
mod router;
mod server;
mod signaling;
mod transport;

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

                    transport::TransportFactory::create_p2p_server(local_ice, Some(room_code)).await
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
                transport::TransportFactory::create_p2p_server(local_ice, config.share_code.clone()).await
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

    // Start HTTP proxy API for external websites (ALWAYS — independent of P2P mode)
    // This is the base API endpoint that external websites use
    let api_handle = {
        let router_clone = router.clone();
        let _transport_clone = transport.clone();
        let bind_addr = config.proxy_api_bind.to_string();

        println!("DEBUG MAIN: bind_addr={}", bind_addr);
        info!(addr = %bind_addr, "Starting HTTP proxy API for external website access");

        tokio::spawn(async move {
            println!("DEBUG PROXY: Starting HTTP proxy API task on {}", bind_addr);
            if let Err(e) = start_http_proxy_api(bind_addr, router_clone, _transport_clone).await {
                error!("HTTP proxy API error: {}", e);
            }
            println!("DEBUG PROXY: HTTP proxy API task ended");
        })
    };

    // Yield to ensure the spawned task gets a chance to start before we block on relay.run()
    tokio::task::yield_now().await;
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

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
    drop(api_handle);

    Ok(())
}

/// HTTP Proxy API for external websites
/// Allows any website (Vercel, etc.) to query the shared database
/// through the P2P tunnel via simple HTTP requests
async fn start_http_proxy_api(
    bind_addr: String,
    router: Arc<router::ShareRouter>,
    _transport: Arc<dyn transport::Transport>,
) -> anyhow::Result<()> {
    // Use eprintln to bypass any tracing/log capture and guarantee visibility
    eprintln!("DEBUG PROXY: bind_addr={}", bind_addr);
    let app_state = ProxyApiState { router };

    let app = Router::new()
        // Health check
        .route("/health", get(proxy_health))
        // CORS preflight
        .route("/api/share/:code/query", options(cors_preflight))
        .route("/api/share/:code/schema", options(cors_preflight))
        .route("/ws/share/:code", options(cors_preflight))
        // Query execution
        .route("/api/share/:code/query", post(proxy_query))
        // Schema fetch
        .route("/api/share/:code/schema", get(proxy_schema))
        // Share info
        .route("/api/share/:code", get(proxy_share_info))
        // WebSocket streaming for real-time queries
        .route("/ws/share/:code", get(ws_proxy_handler))
        .with_state(app_state);

    let addr: SocketAddr = bind_addr.parse()?;
    info!("HTTP proxy API listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

#[derive(Clone)]
struct ProxyApiState {
    router: Arc<router::ShareRouter>,
}

/// CORS preflight response
async fn cors_preflight() -> impl IntoResponse {
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in router::cors_headers() {
        headers.insert(k, v.parse().unwrap());
    }
    (StatusCode::NO_CONTENT, headers)
}

/// Health check for proxy API
async fn proxy_health() -> impl IntoResponse {
    info!("Proxy health check received");
    let body = serde_json::json!({ "status": "ok", "service": "bennett-proxy" });
    (StatusCode::OK, axum::Json(body))
}

/// Execute a query through the proxy
/// External websites POST here with { sql, token }
/// Forwards to engine's POST /api/shares/:code/query endpoint
async fn proxy_query(
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
    Json(req): Json<router::ProxyQueryRequest>,
) -> impl IntoResponse {
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in router::cors_headers() {
        headers.insert(k, v.parse().unwrap());
    }

    // Check if share is active
    if !state.router.is_active(&code) {
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
async fn proxy_schema(
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
) -> impl IntoResponse {
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in router::cors_headers() {
        headers.insert(k, v.parse().unwrap());
    }

    if !state.router.is_active(&code) {
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
    // Token is passed as query param since GET requests don't have bodies
    match state.router.forward_to_engine(
        &code,
        "GET",
        &format!("/api/shares/{}/schema", code),
        None,
        "", // Token will be in URL for GET requests
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

/// Get public share info
async fn proxy_share_info(
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
) -> impl IntoResponse {
    let mut headers = axum::http::HeaderMap::new();
    for (k, v) in router::cors_headers() {
        headers.insert(k, v.parse().unwrap());
    }

    if !state.router.is_active(&code) {
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

/// WebSocket upgrade handler
async fn ws_proxy_handler(
    ws: WebSocketUpgrade,
    Path(code): Path<String>,
    State(state): State<ProxyApiState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_proxy(socket, code, state))
}

/// Handle WebSocket connection for share streaming
async fn handle_ws_proxy(
    socket: WebSocket,
    code: String,
    state: ProxyApiState,
) {
    let (mut sender, mut receiver) = socket.split();
    let mut message_id: u64 = 0;

    info!(share = %code, "WebSocket proxy connection established");

    // Send initial status
    message_id += 1;
    let status_msg = WsProxyResponse::Status {
        connected: state.router.is_active(&code),
        share_code: code.clone(),
        message: "Connected to Bennett relay proxy".to_string(),
        message_id,
    };
    let _ = sender.send(Message::Text(
        serde_json::to_string(&status_msg).unwrap()
    )).await;

    // If share not active, close connection
    if !state.router.is_active(&code) {
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
                let _ = sender.send(Message::Text(
                    serde_json::to_string(&WsProxyResponse::Status {
                        connected: state.router.is_active(&code),
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
