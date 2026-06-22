//! Connect-RPC router
//! Maps HTTP endpoints to service handlers
//! 
//! Connect-RPC protocol:
//! - Unary: POST /bennett.v1.{Service}/{Method}
//! - Server streaming: POST /bennett.v1.{Service}/{Method} (returns ND-JSON stream)

use axum::{
    routing::post,
    Router,
};
use crate::AppState;

use super::{
    query_service,
    schema_service,
    export_service,
};

/// Build Connect-RPC routes
pub fn connect_routes() -> Router<AppState> {
    Router::new()
        // ShareService (also available via REST in api/sharing.rs)
        // QueryService
        .route("/bennett.v1.QueryService/ExecuteQuery", post(query_service::execute_query))
        .route("/bennett.v1.QueryService/ExecuteWrite", post(query_service::execute_write))
        // SchemaService
        .route("/bennett.v1.SchemaService/GetSchema", post(schema_service::get_schema))
        .route("/bennett.v1.SchemaService/GetTableColumns", post(schema_service::get_table_columns))
        .route("/bennett.v1.SchemaService/GetTableIndexes", post(schema_service::get_table_indexes))
        .route("/bennett.v1.SchemaService/GetTableConstraints", post(schema_service::get_table_constraints))
        // ExportService
        .route("/bennett.v1.ExportService/ExportCsv", post(export_service::export_csv))
        .route("/bennett.v1.ExportService/ExportJson", post(export_service::export_json))
        .route("/bennett.v1.ExportService/ExportTableDump", post(export_service::export_table_dump))
}
