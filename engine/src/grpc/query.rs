//! gRPC QueryService implementation
//! Shares business logic with connect_rpc/query_service.rs

use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{info, warn};

use crate::AppState;
use crate::grpc::generated::{
    query_service_server::QueryService,
    ExecuteQueryRequest, ExecuteQueryResponse,
    ExecuteWriteRequest, ExecuteWriteResponse,
    QueryResultRow, Value,
    StreamQueryRequest, QueryChunk,
};
use crate::connect_rpc::{
    validate_share_request, validate_shared_sql, require_write_permission, apply_rls,
};
use crate::grpc::map_error_to_status;

pub struct QueryGrpcService {
    state: AppState,
}

impl QueryGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl QueryService for QueryGrpcService {
    async fn execute_query(
        &self,
        request: Request<ExecuteQueryRequest>,
    ) -> Result<Response<ExecuteQueryResponse>, Status> {
        let req = request.into_inner();
        let start = std::time::Instant::now();
        
        // Validate share
        let validated = validate_share_request(&self.state, &req.share_code, &req.token)
            .await
            .map_err(|e| map_error_to_status(&e))?;
        
        // Validate SQL
        validate_shared_sql(&req.sql, &validated.permission)
            .map_err(|e| map_error_to_status(&e))?;
        
        // Apply RLS
        let sql = apply_rls(&req.sql, validated.rls.as_deref());
        let limit = req.limit.clamp(1, 10000);
        
        // Find database
        let db_instance = {
            let dbs = self.state.databases.lock().unwrap();
            dbs.iter().find(|d| d.id == validated.db_id).cloned()
        };
        
        let db_instance = db_instance.ok_or_else(|| Status::not_found("Database not available"))?;
        
        // Auto-connect
        {
            let mut conn = self.state.connections.lock().await;
            if !conn.is_connected(&db_instance.id) {
                conn.connect(&db_instance).await
                    .map_err(|e| Status::unavailable(format!("Connection failed: {}", e)))?;
            }
        }
        
        // Execute with LIMIT
        let final_sql = if !sql.to_uppercase().contains("LIMIT") {
            format!("{} LIMIT {}", sql, limit)
        } else {
            sql
        };
        
        let result = {
            let conn = self.state.connections.lock().await;
            conn.execute(&db_instance.id, &final_sql).await
                .map_err(|e| Status::internal(format!("Query failed: {}", e)))?
        };
        
        let elapsed = start.elapsed().as_millis() as i64;
        
        // Convert rows to protobuf
        let rows: Vec<QueryResultRow> = result.rows.iter().map(|row| {
            let values: Vec<Value> = row.iter().map(|cell| {
                match cell {
                    serde_json::Value::Null => Value { kind: Some(crate::grpc::generated::value::Kind::NullValue(0)) },
                    serde_json::Value::Bool(b) => Value { kind: Some(crate::grpc::generated::value::Kind::BoolValue(*b)) },
                    serde_json::Value::Number(n) => {
                        if let Some(i) = n.as_i64() {
                            Value { kind: Some(crate::grpc::generated::value::Kind::Int64Value(i)) }
                        } else if let Some(f) = n.as_f64() {
                            Value { kind: Some(crate::grpc::generated::value::Kind::DoubleValue(f)) }
                        } else {
                            Value { kind: Some(crate::grpc::generated::value::Kind::StringValue(n.to_string())) }
                        }
                    }
                    serde_json::Value::String(s) => Value { kind: Some(crate::grpc::generated::value::Kind::StringValue(s.clone())) },
                    _ => Value { kind: Some(crate::grpc::generated::value::Kind::StringValue(cell.to_string())) },
                }
            }).collect();
            
            QueryResultRow { values }
        }).collect();
        
        info!("gRPC query on {}: {} rows in {}ms", req.share_code, result.row_count, elapsed);
        
        Ok(Response::new(ExecuteQueryResponse {
            success: true,
            columns: result.columns,
            rows,
            row_count: result.row_count as i32,
            execution_time_ms: elapsed,
            error: String::new(),
        }))
    }

    async fn execute_write(
        &self,
        request: Request<ExecuteWriteRequest>,
    ) -> Result<Response<ExecuteWriteResponse>, Status> {
        let req = request.into_inner();
        let start = std::time::Instant::now();
        
        // Validate share
        let validated = validate_share_request(&self.state, &req.share_code, &req.token)
            .await
            .map_err(|e| map_error_to_status(&e))?;
        
        // Require write permission
        require_write_permission(&validated.permission)
            .map_err(|e| map_error_to_status(&e))?;
        
        // Validate SQL
        validate_shared_sql(&req.sql, &validated.permission)
            .map_err(|e| map_error_to_status(&e))?;
        
        // Apply RLS
        let sql = apply_rls(&req.sql, validated.rls.as_deref());
        
        // Find database
        let db_instance = {
            let dbs = self.state.databases.lock().unwrap();
            dbs.iter().find(|d| d.id == validated.db_id).cloned()
        };
        
        let db_instance = db_instance.ok_or_else(|| Status::not_found("Database not available"))?;
        
        // Auto-connect
        {
            let mut conn = self.state.connections.lock().await;
            if !conn.is_connected(&db_instance.id) {
                conn.connect(&db_instance).await
                    .map_err(|e| Status::unavailable(format!("Connection failed: {}", e)))?;
            }
        }
        
        // Execute
        let result = {
            let conn = self.state.connections.lock().await;
            conn.execute(&db_instance.id, &sql).await
                .map_err(|e| Status::internal(format!("Write failed: {}", e)))?
        };
        
        let elapsed = start.elapsed().as_millis() as i64;
        
        info!("gRPC write on {}: {} rows in {}ms", req.share_code, result.row_count, elapsed);
        
        Ok(Response::new(ExecuteWriteResponse {
            success: true,
            rows_affected: result.row_count as i64,
            last_insert_id: result.last_insert_id.clone().unwrap_or_default(),
            execution_time_ms: elapsed,
            error: String::new(),
        }))
    }

    type StreamQueryStream = ReceiverStream<Result<QueryChunk, Status>>;

    async fn stream_query(
        &self,
        request: Request<StreamQueryRequest>,
    ) -> Result<Response<Self::StreamQueryStream>, Status> {
        let req = request.into_inner();

        let validated = validate_share_request(&self.state, &req.share_code, &req.token)
            .await
            .map_err(|e| map_error_to_status(&e))?;

        validate_shared_sql(&req.sql, &validated.permission)
            .map_err(|e| map_error_to_status(&e))?;

        let sql = apply_rls(&req.sql, validated.rls.as_deref());
        let chunk_size = 1000; // Default chunk size
        let max_chunks = 100;  // Max 100 chunks = 100k rows

        let db_instance = {
            let dbs = self.state.databases.lock().unwrap();
            dbs.iter().find(|d| d.id == validated.db_id).cloned()
        };

        let db_instance = db_instance.ok_or_else(|| Status::not_found("Database not available"))?;

        {
            let mut conn = self.state.connections.lock().await;
            if !conn.is_connected(&db_instance.id) {
                conn.connect(&db_instance).await
                    .map_err(|e| Status::unavailable(format!("Connection failed: {}", e)))?;
            }
        }

        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let state = self.state.clone();
        let db_id = db_instance.id.clone();

        tokio::spawn(async move {
            let mut offset = 0;
            let mut chunk_index = 0;
            let mut total_rows = 0;

            loop {
                if chunk_index >= max_chunks {
                    break;
                }

                let chunk_sql = format!("{} LIMIT {} OFFSET {}", sql, chunk_size, offset);

                let result = {
                    let conn = state.connections.lock().await;
                    match conn.execute(&db_id, &chunk_sql).await {
                        Ok(r) => r,
                        Err(e) => {
                            let _ = tx.send(Err(Status::internal(format!("Query failed: {}", e)))).await;
                            break;
                        }
                    }
                };

                if result.rows.is_empty() {
                    break;
                }

                let rows: Vec<QueryResultRow> = result.rows.iter().map(|row| {
                    let values: Vec<Value> = row.iter().map(|cell| {
                        match cell {
                            serde_json::Value::Null => Value { kind: Some(crate::grpc::generated::value::Kind::NullValue(0)) },
                            serde_json::Value::Bool(b) => Value { kind: Some(crate::grpc::generated::value::Kind::BoolValue(*b)) },
                            serde_json::Value::Number(n) => {
                                if let Some(i) = n.as_i64() {
                                    Value { kind: Some(crate::grpc::generated::value::Kind::Int64Value(i)) }
                                } else if let Some(f) = n.as_f64() {
                                    Value { kind: Some(crate::grpc::generated::value::Kind::DoubleValue(f)) }
                                } else {
                                    Value { kind: Some(crate::grpc::generated::value::Kind::StringValue(n.to_string())) }
                                }
                            }
                            serde_json::Value::String(s) => Value { kind: Some(crate::grpc::generated::value::Kind::StringValue(s.clone())) },
                            _ => Value { kind: Some(crate::grpc::generated::value::Kind::StringValue(cell.to_string())) },
                        }
                    }).collect();
                    QueryResultRow { values }
                }).collect();

                total_rows += rows.len();
                let is_last = rows.len() < chunk_size;

                if tx.send(Ok(QueryChunk {
                    columns: result.columns.clone(),
                    rows,
                    is_last,
                    total_rows: total_rows as i64,
                    chunk_index,
                })).await.is_err() {
                    break;
                }

                if is_last {
                    break;
                }

                offset += chunk_size;
                chunk_index += 1;
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
