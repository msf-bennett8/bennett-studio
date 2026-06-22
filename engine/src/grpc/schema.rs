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
        _request: Request<GetTableColumnsRequest>,
    ) -> Result<Response<GetTableColumnsResponse>, Status> {
        // TODO: Implement direct column fetch
        Ok(Response::new(GetTableColumnsResponse {
            success: false,
            columns: vec![],
            error: "Direct column fetch not yet implemented. Use GetSchema.".to_string(),
        }))
    }

    async fn get_table_indexes(
        &self,
        _request: Request<GetTableIndexesRequest>,
    ) -> Result<Response<GetTableIndexesResponse>, Status> {
        Ok(Response::new(GetTableIndexesResponse {
            success: false,
            indexes: vec![],
            error: "Not yet implemented".to_string(),
        }))
    }

    async fn get_table_constraints(
        &self,
        _request: Request<GetTableConstraintsRequest>,
    ) -> Result<Response<GetTableConstraintsResponse>, Status> {
        Ok(Response::new(GetTableConstraintsResponse {
            success: false,
            constraints: vec![],
            error: "Not yet implemented".to_string(),
        }))
    }
}
