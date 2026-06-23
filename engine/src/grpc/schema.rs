//! gRPC SchemaService implementation
//! Shares business logic with connect_rpc/schema_service.rs

use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use tracing::info;

use crate::AppState;
use crate::grpc::generated::{
    schema_service_server::SchemaService,
    GetSchemaRequest, GetSchemaResponse,
    TableSchema, ColumnSchema, IndexSchema, ConstraintSchema,
    GetTableColumnsRequest, GetTableColumnsResponse,
    GetTableIndexesRequest, GetTableIndexesResponse,
    GetTableConstraintsRequest, GetTableConstraintsResponse,
    SchemaUpdate,
};
use crate::connect_rpc::validate_share_request;
use crate::grpc::map_error_to_status;

pub struct SchemaGrpcService {
    state: AppState,
}

impl SchemaGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl SchemaService for SchemaGrpcService {
    async fn get_schema(
        &self,
        request: Request<GetSchemaRequest>,
    ) -> Result<Response<GetSchemaResponse>, Status> {
        let req = request.into_inner();
        let start = std::time::Instant::now();
        
        // Validate
        let validated = validate_share_request(&self.state, &req.share_code, &req.token, None)
            .await
            .map_err(|e| map_error_to_status(&format!("{:?}", e)))?;
        
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
        
        // Get schema
        let schema_result = {
            let conn = self.state.connections.lock().await;
            conn.get_schema(&db_instance.id).await
                .map_err(|e| Status::internal(format!("Schema fetch failed: {}", e)))?
        };
        
        // Convert to protobuf
        let tables: Vec<TableSchema> = schema_result.into_iter().map(|table_info| {
            TableSchema {
                name: table_info.name,
                columns: table_info.columns.into_iter().map(|col| ColumnSchema {
                    name: col.name,
                    data_type: col.data_type,
                    nullable: col.nullable,
                    default_value: String::new(),
                    is_primary_key: false,
                    is_foreign_key: false,
                    foreign_key_reference: String::new(),
                    comment: String::new(),
                }).collect(),
                indexes: vec![],
                constraints: vec![],
                estimated_row_count: 0,
                table_size: String::new(),
            }
        }).collect();
        
        let elapsed = start.elapsed().as_millis() as i64;
        info!("gRPC schema for {}: {} tables in {}ms", req.share_code, tables.len(), elapsed);
        
        Ok(Response::new(GetSchemaResponse {
            success: true,
            tables,
            database_name: db_instance.name,
            database_type: db_instance.db_type,
            database_version: db_instance.version,
        }))
    }

    type StreamSchemaUpdatesStream = ReceiverStream<Result<SchemaUpdate, Status>>;

    async fn stream_schema_updates(
        &self,
        request: Request<GetSchemaRequest>,
    ) -> Result<Response<Self::StreamSchemaUpdatesStream>, Status> {
        let req = request.into_inner();

        // Validate share
        let validated = validate_share_request(&self.state, &req.share_code, &req.token, None)
            .await
            .map_err(|e| map_error_to_status(&format!("{:?}", e)))?;

        let db_id = validated.db_id.clone();
        let share_code = req.share_code.clone();

        // Find database
        let db_instance = {
            let dbs = self.state.databases.lock().unwrap();
            dbs.iter().find(|d| d.id == db_id).cloned()
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

        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let connections = self.state.connections.clone();
        let db_instance_clone = db_instance.clone();

        // Spawn polling task with proper cancellation and TTL
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
            let mut last_schema_hash: Option<u64> = None;
            let mut consecutive_errors = 0u8;
            const MAX_ERRORS: u8 = 5;

            // Send initial FULL_REFRESH immediately
            let initial_schema = {
                let conn = connections.lock().await;
                match conn.get_schema(&db_instance_clone.id).await {
                    Ok(s) => s,
                    Err(e) => {
                        let _ = tx.send(Err(Status::unavailable(
                            format!("Schema fetch failed: {}", e)
                        ))).await;
                        return;
                    }
                }
            };

            let initial_hash = compute_schema_hash(&initial_schema);
            last_schema_hash = Some(initial_hash);

            let _tables: Vec<TableSchema> = initial_schema.into_iter().map(|t| TableSchema {
                name: t.name,
                columns: t.columns.into_iter().map(|c| ColumnSchema {
                    name: c.name,
                    data_type: c.data_type,
                    nullable: c.nullable,
                    default_value: String::new(),
                    is_primary_key: false,
                    is_foreign_key: false,
                    foreign_key_reference: String::new(),
                    comment: String::new(),
                }).collect(),
                indexes: vec![],
                constraints: vec![],
                estimated_row_count: 0,
                table_size: String::new(),
            }).collect();

            if tx.send(Ok(SchemaUpdate {
                r#type: 0, // FULL_REFRESH
                table: None,
                removed_table_name: String::new(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            })).await.is_err() {
                return; // Client disconnected
            }

            // Polling loop with 24h TTL max
            let start_time = std::time::Instant::now();
            let max_duration = std::time::Duration::from_secs(24 * 60 * 60);

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if start_time.elapsed() > max_duration {
                            let _ = tx.send(Err(Status::deadline_exceeded(
                                "Schema stream session expired (24h limit)"
                            ))).await;
                            break;
                        }

                        if consecutive_errors >= MAX_ERRORS {
                            let _ = tx.send(Err(Status::unavailable(
                                "Too many consecutive schema fetch errors"
                            ))).await;
                            break;
                        }

                        let current_schema = {
                            let conn = connections.lock().await;
                            match conn.get_schema(&db_instance_clone.id).await {
                                Ok(s) => s,
                                Err(e) => {
                                    consecutive_errors += 1;
                                    tracing::warn!("Schema poll error for {}: {}", share_code, e);
                                    continue;
                                }
                            }
                        };

                        consecutive_errors = 0; // Reset on success
                        let current_hash = compute_schema_hash(&current_schema);

                        if Some(current_hash) != last_schema_hash {
                            last_schema_hash = Some(current_hash);
                            info!("Schema change detected for share {}, sending FULL_REFRESH", share_code);

                            if tx.send(Ok(SchemaUpdate {
                                r#type: 0, // FULL_REFRESH (simplified — could diff for granular updates)
                                table: None,
                                removed_table_name: String::new(),
                                timestamp: chrono::Utc::now().to_rfc3339(),
                            })).await.is_err() {
                                break; // Client disconnected
                            }
                        }
                    }
                    // Channel closed = client disconnected
                    else => {
                        tracing::debug!("Schema stream channel closed for share {}", share_code);
                        break;
                    }
                }
            }

            tracing::info!("Schema update stream ended for share {}", share_code);
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_table_columns(
        &self,
        request: Request<GetTableColumnsRequest>,
    ) -> Result<Response<GetTableColumnsResponse>, Status> {
        let req = request.into_inner();

        let validated = validate_share_request(&self.state, &req.share_code, &req.token, None)
            .await
            .map_err(|e| map_error_to_status(&format!("{:?}", e)))?;

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

        let columns = {
            let conn = self.state.connections.lock().await;
            conn.get_table_columns(&db_instance.id, &req.table_name).await
                .map_err(|e| Status::internal(format!("Failed to fetch columns: {}", e)))?
        };

        let columns: Vec<ColumnSchema> = columns.into_iter().map(|col| ColumnSchema {
            name: col.name,
            data_type: col.data_type,
            nullable: col.nullable,
            default_value: String::new(),
            is_primary_key: false,
            is_foreign_key: false,
            foreign_key_reference: String::new(),
            comment: String::new(),
        }).collect();

        Ok(Response::new(GetTableColumnsResponse {
            success: true,
            columns,
        }))
    }

    async fn get_table_indexes(
        &self,
        request: Request<GetTableIndexesRequest>,
    ) -> Result<Response<GetTableIndexesResponse>, Status> {
        let req = request.into_inner();

        let validated = validate_share_request(&self.state, &req.share_code, &req.token, None)
            .await
            .map_err(|e| map_error_to_status(&format!("{:?}", e)))?;

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

        let indexes = {
            let conn = self.state.connections.lock().await;
            conn.get_table_indexes(&db_instance.id, &req.table_name).await
                .map_err(|e| Status::internal(format!("Failed to fetch indexes: {}", e)))?
        };

        let indexes: Vec<IndexSchema> = indexes.into_iter().map(|i| IndexSchema {
            name: i.name,
            columns: i.columns,
            index_type: i.index_type,
            is_unique: i.is_unique,
            is_primary: i.is_primary,
        }).collect();

        Ok(Response::new(GetTableIndexesResponse {
            success: true,
            indexes,
        }))
    }

    async fn get_table_constraints(
        &self,
        request: Request<GetTableConstraintsRequest>,
    ) -> Result<Response<GetTableConstraintsResponse>, Status> {
        let req = request.into_inner();

        let validated = validate_share_request(&self.state, &req.share_code, &req.token, None)
            .await
            .map_err(|e| map_error_to_status(&format!("{:?}", e)))?;

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

        let constraints = {
            let conn = self.state.connections.lock().await;
            conn.get_table_constraints(&db_instance.id, &req.table_name).await
                .map_err(|e| Status::internal(format!("Failed to fetch constraints: {}", e)))?
        };

        let constraints: Vec<ConstraintSchema> = constraints.into_iter().map(|c| ConstraintSchema {
            name: c.name,
            constraint_type: c.constraint_type,
            columns: c.columns,
            definition: c.definition.unwrap_or_default(),
        }).collect();

        Ok(Response::new(GetTableConstraintsResponse {
            success: true,
            constraints,
        }))
    }
}

/// Compute a hash of schema for change detection
/// Uses table names + column names + types
fn compute_schema_hash(schema: &[crate::control_plane::connection::manager::TableInfo]) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    for table in schema {
        table.name.hash(&mut hasher);
        for col in &table.columns {
            col.name.hash(&mut hasher);
            col.data_type.hash(&mut hasher);
            col.nullable.hash(&mut hasher);
        }
    }
    hasher.finish()
}
