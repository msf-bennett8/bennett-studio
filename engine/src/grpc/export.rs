//! gRPC ExportService implementation
//! Shares business logic with connect_rpc/export_service.rs

use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;
use tracing::info;

use crate::AppState;
use crate::grpc::generated::{
    export_service_server::ExportService,
    ExportRequest, ExportChunk,
    ExportTableRequest,
};
use crate::connect_rpc::{validate_share_request, validate_shared_sql};
use crate::grpc::map_error_to_status;

pub struct ExportGrpcService {
    state: AppState,
}

impl ExportGrpcService {
    pub fn new(state: AppState) -> Self {
        Self { state }
    }
}

#[tonic::async_trait]
impl ExportService for ExportGrpcService {
    type ExportCsvStream = ReceiverStream<Result<ExportChunk, Status>>;
    type ExportJsonStream = ReceiverStream<Result<ExportChunk, Status>>;
    type ExportParquetStream = ReceiverStream<Result<ExportChunk, Status>>;
    type ExportTableDumpStream = ReceiverStream<Result<ExportChunk, Status>>;

    async fn export_csv(
        &self,
        request: Request<ExportRequest>,
    ) -> Result<Response<Self::ExportCsvStream>, Status> {
        self.stream_export(request.into_inner(), "csv").await
    }

    async fn export_json(
        &self,
        request: Request<ExportRequest>,
    ) -> Result<Response<Self::ExportJsonStream>, Status> {
        self.stream_export(request.into_inner(), "json").await
    }

    async fn export_parquet(
        &self,
        _request: Request<ExportRequest>,
    ) -> Result<Response<Self::ExportParquetStream>, Status> {
        // TODO: Implement Parquet export
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        tokio::spawn(async move {
            let _ = tx.send(Err(Status::unimplemented("Parquet export not yet implemented"))).await;
        });
        Ok(Response::new(ReceiverStream::new(rx)))
    }

    async fn export_table_dump(
        &self,
        request: Request<ExportTableRequest>,
    ) -> Result<Response<Self::ExportTableDumpStream>, Status> {
        let req = request.into_inner();
        let export_req = ExportRequest {
            share_code: req.share_code,
            token: req.token,
            sql: format!(r#"SELECT * FROM "{}""#, req.table_name),
            format: req.format,
            include_headers: true,
        };
        self.stream_export(export_req, &req.format).await
    }
}

impl ExportGrpcService {
    async fn stream_export(
        &self,
        req: ExportRequest,
        format: &str,
    ) -> Result<ReceiverStream<Result<ExportChunk, Status>>, Status> {
        let start = std::time::Instant::now();
        
        // Validate
        let validated = validate_share_request(&self.state, &req.share_code, &req.token)
            .await
            .map_err(|e| map_error_to_status(&e))?;
        
        validate_shared_sql(&req.sql, &validated.permission)
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
        
        // Execute
        let result = {
            let conn = self.state.connections.lock().await;
            conn.execute(&db_instance.id, &req.sql).await
                .map_err(|e| Status::internal(format!("Export query failed: {}", e)))?
        };
        
        let elapsed = start.elapsed().as_millis() as i64;
        
        // Format output
        let data = match format {
            "csv" => self.format_csv(&result.columns, &result.rows, req.include_headers),
            "json" => self.format_json(&result.columns, &result.rows),
            _ => return Err(Status::invalid_argument(format!("Unsupported format: {}", format))),
        };
        
        info!("gRPC export on {}: {} rows as {} in {}ms", req.share_code, result.row_count, format, elapsed);
        
        // Single chunk for now - TODO: stream large results
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        let chunk = ExportChunk {
            data: data.into_bytes(),
            is_last: true,
            total_rows: result.row_count as i64,
            chunk_index: 0,
        };
        
        tokio::spawn(async move {
            let _ = tx.send(Ok(chunk)).await;
        });
        
        Ok(ReceiverStream::new(rx))
    }
    
    fn format_csv(
        &self,
        columns: &[String],
        rows: &[Vec<serde_json::Value>],
        include_headers: bool,
    ) -> String {
        let mut output = String::new();
        
        if include_headers {
            output.push_str(&columns.join(","));
            output.push('\n');
        }
        
        for row in rows {
            let values: Vec<String> = row.iter().map(|v| {
                match v {
                    serde_json::Value::Null => String::new(),
                    serde_json::Value::String(s) => {
                        if s.contains(',') || s.contains('"') || s.contains('\n') {
                            format!("\"{}\"", s.replace("\"", "\"\""))
                        } else {
                            s.clone()
                        }
                    }
                    other => other.to_string(),
                }
            }).collect();
            output.push_str(&values.join(","));
            output.push('\n');
        }
        
        output
    }
    
    fn format_json(
        &self,
        columns: &[String],
        rows: &[Vec<serde_json::Value>],
    ) -> String {
        let mut objects = Vec::new();
        
        for row in rows {
            let mut obj = serde_json::Map::new();
            for (i, col) in columns.iter().enumerate() {
                let value = row.get(i).cloned().unwrap_or(serde_json::Value::Null);
                obj.insert(col.clone(), value);
            }
            objects.push(serde_json::Value::Object(obj));
        }
        
        serde_json::to_string_pretty(&objects).unwrap_or_default()
    }
}
