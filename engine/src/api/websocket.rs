use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    response::IntoResponse,
};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::interval;
use tracing::{info, warn};

use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsRequest {
    SubscribeLogs { database_id: String },
    ExecuteQuery { database_id: String, sql: String },
    Ping,
    // Phase 6: Reconnection support
    Reconnect { session_id: String, last_message_id: u64 },
    Ack { message_id: u64 },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsResponse {
    LogLine { database_id: String, line: String, timestamp: String, message_id: u64 },
    QueryResult { database_id: String, columns: Vec<String>, rows: Vec<Vec<serde_json::Value>>, row_count: usize, execution_time_ms: u64, message_id: u64 },
    QueryError { database_id: String, error: String, message_id: u64 },
    HealthUpdate { database_id: String, status: String, uptime_seconds: u64, message_id: u64 },
    Pong,
    Error { message: String },
    // Phase 6: Reconnection support
    ReconnectAck { session_id: String, last_message_id: u64, missed_messages: Vec<WsResponse> },
    Hello { session_id: String, server_time: String },
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Path(database_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, database_id, state))
}

async fn handle_socket(socket: WebSocket, database_id: String, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    
    // Generate session ID for reconnection
    let session_id = format!("ws-{}", uuid::Uuid::new_v4());
    let mut message_counter: u64 = 0;
    
    // Send hello with session ID
    let _ = sender.send(Message::Text(
        serde_json::to_string(&WsResponse::Hello {
            session_id: session_id.clone(),
            server_time: chrono::Utc::now().to_rfc3339(),
        }).unwrap()
    )).await;

    // Send initial connection confirmation
    let _ = sender.send(Message::Text(
        serde_json::to_string(&WsResponse::Pong).unwrap()
    )).await;

    let mut log_interval = interval(Duration::from_secs(2));
    let mut health_interval = interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            // Handle incoming messages from client
            msg = receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<WsRequest>(&text) {
                            Ok(WsRequest::Ping) => {
                                let _ = sender.send(Message::Text(
                                    serde_json::to_string(&WsResponse::Pong).unwrap()
                                )).await;
                            }
                            Ok(WsRequest::Reconnect { session_id: _, last_message_id: _ }) => {
                                // TODO: Implement message replay from buffer
                                let _ = sender.send(Message::Text(
                                    serde_json::to_string(&WsResponse::ReconnectAck {
                                        session_id: session_id.clone(),
                                        last_message_id: message_counter,
                                        missed_messages: vec![],
                                    }).unwrap()
                                )).await;
                            }
                            Ok(WsRequest::Ack { message_id }) => {
                                // Client acknowledged receipt, can remove from buffer
                                debug!("Client acked message {}", message_id);
                            }
                            Ok(WsRequest::ExecuteQuery { database_id: db_id, sql }) => {
                                let start = std::time::Instant::now();
                                let instance = {
                                    let db = state.databases.lock().unwrap();
                                    db.iter().find(|d| d.id == db_id).cloned()
                                };

                                let instance = match instance {
                                    Some(i) => i,
                                    None => {
                                        let _ = sender.send(Message::Text(
                                            serde_json::to_string(&WsResponse::QueryError {
                                                database_id: db_id,
                                                error: "Database not found".to_string(),
                                            }).unwrap()
                                        )).await;
                                        continue;
                                    }
                                };

                                // Auto-connect
                                let needs_connect = {
                                    let conn = state.connections.lock().await;
                                    !conn.is_connected(&db_id)
                                };

                                if needs_connect {
                                    let mut conn = state.connections.lock().await;
                                    if let Err(e) = conn.connect(&instance).await {
                                        let _ = sender.send(Message::Text(
                                            serde_json::to_string(&WsResponse::QueryError {
                                                database_id: db_id,
                                                error: format!("Connection failed: {}", e),
                                            }).unwrap()
                                        )).await;
                                        continue;
                                    }
                                }

                                let result = {
                                    let conn = state.connections.lock().await;
                                    conn.execute(&db_id, &sql).await
                                };

                                match result {
                                    Ok(query_result) => {
                                        let elapsed = start.elapsed().as_millis() as u64;
                                        message_counter += 1;
                                        let _ = sender.send(Message::Text(
                                            serde_json::to_string(&WsResponse::QueryResult {
                                                database_id: db_id,
                                                columns: query_result.columns,
                                                rows: query_result.rows,
                                                row_count: query_result.row_count,
                                                execution_time_ms: elapsed,
                                                message_id: message_counter,
                                            }).unwrap()
                                        )).await;
                                    }
                                    Err(e) => {
                                        let _ = sender.send(Message::Text(
                                            serde_json::to_string(&WsResponse::QueryError {
                                                database_id: db_id,
                                                error: format!("Query failed: {}", e),
                                            }).unwrap()
                                        )).await;
                                    }
                                }
                            }
                            Ok(WsRequest::SubscribeLogs { database_id: db_id }) => {
                                info!("Client subscribed to logs for {}", db_id);
                            }
                            Err(e) => {
                                let _ = sender.send(Message::Text(
                                    serde_json::to_string(&WsResponse::Error {
                                        message: format!("Invalid message: {}", e),
                                    }).unwrap()
                                )).await;
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        info!("WebSocket client disconnected for {}", database_id);
                        break;
                    }
                    Some(Err(e)) => {
                        warn!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // Stream logs periodically
            _ = log_interval.tick() => {
                let instance = {
                    let db = state.databases.lock().unwrap();
                    db.iter().find(|d| d.id == database_id).cloned()
                };

                if let Some(instance) = instance {
                    if let Some(ref container_id) = instance.container_id {
                        match state.docker.get_logs(container_id).await {
                            Ok(logs) if !logs.is_empty() => {
                                for line in logs.lines().rev().take(5) {
                                    let _ = sender.send(Message::Text(
                                        serde_json::to_string(&WsResponse::LogLine {
                                            database_id: database_id.clone(),
                                            line: line.to_string(),
                                            timestamp: chrono::Local::now().to_rfc3339(),
                                        }).unwrap()
                                    )).await;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }

            // Send health updates
            _ = health_interval.tick() => {
                let instance = {
                    let db = state.databases.lock().unwrap();
                    db.iter().find(|d| d.id == database_id).cloned()
                };

                if let Some(instance) = instance {
                    let status = format!("{:?}", instance.status);
                    let _ = sender.send(Message::Text(
                        serde_json::to_string(&WsResponse::HealthUpdate {
                            database_id: database_id.clone(),
                            status,
                            uptime_seconds: 0,
                        }).unwrap()
                    )).await;
                }
            }
        }
    }
}