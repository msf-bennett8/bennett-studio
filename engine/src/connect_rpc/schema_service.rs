//! Connect-RPC SchemaService implementation
//! GetSchema, StreamSchemaUpdates, GetTableColumns, GetTableIndexes, GetTableConstraints

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::{info, warn};

use crate::AppState;
use crate::connect_rpc::{
    connect_error, connect_response, validate_share_request,
    parse_connect_request,
};

// ============================================================================
// Request/Response Types
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct GetSchemaRequest {
    pub share_code: String,
    pub token: String,
}

#[derive(Debug, Serialize)]
pub struct GetSchemaResponse {
    pub success: bool,
    pub tables: Vec<TableSchema>,
    pub database_name: String,
    pub database_type: String,
    pub database_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnSchema>,
    pub indexes: Vec<IndexSchema>,
    pub constraints: Vec<ConstraintSchema>,
    pub estimated_row_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_size: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ColumnSchema {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_value: Option<String>,
    pub is_primary_key: bool,
    pub is_foreign_key: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foreign_key_reference: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct IndexSchema {
    pub name: String,
    pub columns: Vec<String>,
    pub index_type: String,
    pub is_unique: bool,
    pub is_primary: bool,
}

#[derive(Debug, Serialize, Clone)]
pub struct ConstraintSchema {
    pub name: String,
    pub constraint_type: String,
    pub columns: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetTableColumnsRequest {
    pub share_code: String,
    pub token: String,
    pub table_name: String,
}

#[derive(Debug, Serialize)]
pub struct GetTableColumnsResponse {
    pub success: bool,
    pub columns: Vec<ColumnSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetTableIndexesRequest {
    pub share_code: String,
    pub token: String,
    pub table_name: String,
}

#[derive(Debug, Serialize)]
pub struct GetTableIndexesResponse {
    pub success: bool,
    pub indexes: Vec<IndexSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct GetTableConstraintsRequest {
    pub share_code: String,
    pub token: String,
    pub table_name: String,
}

#[derive(Debug, Serialize)]
pub struct GetTableConstraintsResponse {
    pub success: bool,
    pub constraints: Vec<ConstraintSchema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

// ============================================================================
// Handlers
// ============================================================================

/// POST /bennett.v1.SchemaService/GetSchema
pub async fn get_schema(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let req: GetSchemaRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    
    let start = std::time::Instant::now();
    
    // Validate share
    let validated = match validate_share_request(&state, &req.share_code, &req.token).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    
    // Find database
    let db_instance = {
        let dbs = state.databases.lock().unwrap();
        dbs.iter().find(|d| d.id == validated.db_id).cloned()
    };
    
    let db_instance = match db_instance {
        Some(d) => d,
        None => return connect_error("not_found", "Database not available"),
    };
    
    // Auto-connect
    {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&db_instance.id) {
            if let Err(e) = conn.connect(&db_instance).await {
                return connect_error("unavailable", &format!("Connection failed: {}", e));
            }
        }
    }
    
    // Get schema from connection manager
    let schema_result = {
        let conn = state.connections.lock().await;
        match conn.get_schema(&db_instance.id).await {
            Ok(s) => s,
            Err(e) => {
                warn!("Schema fetch failed: {}", e);
                return connect_response(GetSchemaResponse {
                    success: false,
                    tables: vec![],
                    database_name: db_instance.name.clone(),
                    database_type: db_instance.db_type.clone(),
                    database_version: db_instance.version.clone(),
                    error: Some(format!("Schema fetch failed: {}", e)),
                });
            }
        }
    };
    
    // Convert to our schema format
    let tables: Vec<TableSchema> = schema_result.into_iter().map(|table_info| {
        TableSchema {
            name: table_info.name,
            columns: table_info.columns.into_iter().map(|col| ColumnSchema {
                name: col.name,
                data_type: col.data_type,
                nullable: col.nullable,
                default_value: None,
                is_primary_key: false, // TODO: Detect from schema
                is_foreign_key: false,
                foreign_key_reference: None,
                comment: None,
            }).collect(),
            indexes: vec![], // TODO: Fetch indexes
            constraints: vec![], // TODO: Fetch constraints
            estimated_row_count: 0,
            table_size: None,
        }
    }).collect();
    
    let elapsed = start.elapsed().as_millis() as i64;
    info!("Schema fetched for share {}: {} tables in {}ms", req.share_code, tables.len(), elapsed);
    
    connect_response(GetSchemaResponse {
        success: true,
        tables,
        database_name: db_instance.name,
        database_type: db_instance.db_type,
        database_version: db_instance.version,
        error: None,
    })
}

/// POST /bennett.v1.SchemaService/GetTableColumns
pub async fn get_table_columns(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let req: GetTableColumnsRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    
    // Validate share
    let validated = match validate_share_request(&state, &req.share_code, &req.token).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };
    
    // Get full schema then filter
    // TODO: Optimize with direct column query
    let schema_resp = get_schema(State(state), Json(json!({
        "shareCode": req.share_code,
        "token": req.token
    }))).await;
    
    // Extract columns from response
    // For now, return error - full implementation requires parsing the response
    connect_response(GetTableColumnsResponse {
        success: false,
        columns: vec![],
        error: Some("Direct column fetch not yet implemented. Use GetSchema.".to_string()),
    })
}

/// POST /bennett.v1.SchemaService/GetTableIndexes
pub async fn get_table_indexes(
    State(_state): State<AppState>,
    Json(_body): Json<serde_json::Value>,
) -> Response {
    // TODO: Implement index fetching
    connect_response(GetTableIndexesResponse {
        success: false,
        indexes: vec![],
        error: Some("Index fetching not yet implemented".to_string()),
    })
}

/// POST /bennett.v1.SchemaService/GetTableConstraints
pub async fn get_table_constraints(
    State(_state): State<AppState>,
    Json(_body): Json<serde_json::Value>,
) -> Response {
    // TODO: Implement constraint fetching
    connect_response(GetTableConstraintsResponse {
        success: false,
        constraints: vec![],
        error: Some("Constraint fetching not yet implemented".to_string()),
    })
}

/// TODO: Phase 2 - Implement StreamSchemaUpdates for real-time autocomplete
/// TODO: Phase 3 - Implement column-level permission filtering
/// TODO: Phase 3 - Implement schema caching with TTL
