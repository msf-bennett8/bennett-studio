//! gRPC SchemaService implementation
//! Shares business logic with connect_rpc/schema_service.rs

use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{info, warn};

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
        let validated = validate_share_request(&self.state, &req.share_code, &req.token)
            .await
            .map_err(|e| map_error_to_status(&e))?;
        
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
            error: String::new(),
        }))
    }

    type StreamSchemaUpdatesStream = ReceiverStream<Result<SchemaUpdate, Status>>;

    async fn stream_schema_updates(
        &self,
        _request: Request<GetSchemaRequest>,
    ) -> Result<Response<Self::StreamSchemaUpdatesStream>, Status> {
        // TODO: Implement streaming schema updates for real-time autocomplete
        // Schema change streaming requires database trigger/monitoring infrastructure
        // Placeholder: sends single full-refresh event
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        
        tokio::spawn(async move {
            // Placeholder - would push schema changes as they happen
            let _ = tx.send(Ok(SchemaUpdate {
                r#type: 0, // FULL_REFRESH
                table: None,
                removed_table_name: String::new(),
                timestamp: chrono::Utc::now().to_rfc3339(),
            })).await;
        });
        
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn get_table_columns(
        &self,
        request: Request<GetTableColumnsRequest>,
    ) -> Result<Response<GetTableColumnsResponse>, Status> {
        let req = request.into_inner();

        let validated = validate_share_request(&self.state, &req.share_code, &req.token)
            .await
            .map_err(|e| map_error_to_status(&e))?;

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
            error: String::new(),
        }))
    }

    async fn get_table_indexes(
        &self,
        request: Request<GetTableIndexesRequest>,
    ) -> Result<Response<GetTableIndexesResponse>, Status> {
        let req = request.into_inner();

        let validated = validate_share_request(&self.state, &req.share_code, &req.token)
            .await
            .map_err(|e| map_error_to_status(&e))?;

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
            error: String::new(),
        }))
    }

    async fn get_table_constraints(
        &self,
        request: Request<GetTableConstraintsRequest>,
    ) -> Result<Response<GetTableConstraintsResponse>, Status> {
        let req = request.into_inner();

        let validated = validate_share_request(&self.state, &req.share_code, &req.token)
            .await
            .map_err(|e| map_error_to_status(&e))?;

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
            error: String::new(),
        }))
    }
}
