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
            last_insert_id: String::new(),
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
        
        // Validate
        let validated = validate_share_request(&self.state, &req.share_code, &req.token)
            .await
            .map_err(|e| map_error_to_status(&e))?;
        
        validate_shared_sql(&req.sql, &validated.permission)
            .map_err(|e| map_error_to_status(&e))?;
        
        // TODO: Implement streaming with chunked results
        // For now, return single chunk
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        
        tokio::spawn(async move {
            // Placeholder - full implementation would stream rows in chunks
            let _ = tx.send(Ok(QueryChunk {
                rows: vec![],
                is_last: true,
                total_rows: 0,
                chunk_index: 0,
            })).await;
        });
        
        Ok(Response::new(ReceiverStream::new(rx)))
    }
}
