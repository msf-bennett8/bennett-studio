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
    request: axum::extract::Request,
) -> Response {
    let client_ip = request.extensions().get::<crate::api::middleware::ClientIp>().map(|c| c.0);
    let req: GetSchemaRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };
    
    let start = std::time::Instant::now();
    
    // Validate
    let validated = match validate_share_request(&state, &req.share_code, &req.token, client_ip).await {
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

    // Convert to our schema format with real metadata
    let mut tables = Vec::new();
    for table_info in schema_result {
        let table_name = table_info.name.clone();
        
        // Fetch indexes and constraints for each table
        let (indexes, constraints) = {
            let conn = state.connections.lock().await;
            let idx = conn.get_table_indexes(&db_instance.id, &table_name).await.unwrap_or_default();
            let cst = conn.get_table_constraints(&db_instance.id, &table_name).await.unwrap_or_default();
            (idx, cst)
        };
        
        // Detect primary key from constraints
        let pk_columns: Vec<String> = constraints.iter()
            .filter(|c| c.constraint_type == "PRIMARY KEY")
            .flat_map(|c| c.columns.clone())
            .collect();

        tables.push(TableSchema {
            name: table_info.name,
            columns: table_info.columns.into_iter().map(|col| ColumnSchema {
                name: col.name.clone(),
                data_type: col.data_type,
                nullable: col.nullable,
                default_value: None,
                is_primary_key: pk_columns.contains(&col.name),
                is_foreign_key: false,
                foreign_key_reference: None,
                comment: None,
            }).collect(),
            indexes: indexes.into_iter().map(|i| IndexSchema {
                name: i.name,
                columns: i.columns,
                index_type: i.index_type,
                is_unique: i.is_unique,
                is_primary: i.is_primary,
            }).collect(),
            constraints: constraints.into_iter().map(|c| ConstraintSchema {
                name: c.name,
                constraint_type: c.constraint_type,
                columns: c.columns,
                definition: c.definition,
            }).collect(),
            estimated_row_count: 0,
            table_size: None,
        });
    }
    
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

    // Get columns directly
    let columns = {
        let conn = state.connections.lock().await;
        match conn.get_table_columns(&db_instance.id, &req.table_name).await {
            Ok(cols) => cols.into_iter().map(|col| ColumnSchema {
                name: col.name,
                data_type: col.data_type,
                nullable: col.nullable,
                default_value: None,
                is_primary_key: false, // Will be detected separately
                is_foreign_key: false,
                foreign_key_reference: None,
                comment: None,
            }).collect(),
            Err(e) => {
                return connect_response(GetTableColumnsResponse {
                    success: false,
                    columns: vec![],
                    error: Some(format!("Failed to fetch columns: {}", e)),
                });
            }
        }
    };

    connect_response(GetTableColumnsResponse {
        success: true,
        columns,
        error: None,
    })
}

/// POST /bennett.v1.SchemaService/GetTableIndexes
pub async fn get_table_indexes(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let req: GetTableIndexesRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    let validated = match validate_share_request(&state, &req.share_code, &req.token).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let db_instance = {
        let dbs = state.databases.lock().unwrap();
        dbs.iter().find(|d| d.id == validated.db_id).cloned()
    };

    let db_instance = match db_instance {
        Some(d) => d,
        None => return connect_error("not_found", "Database not available"),
    };

    {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&db_instance.id) {
            if let Err(e) = conn.connect(&db_instance).await {
                return connect_error("unavailable", &format!("Connection failed: {}", e));
            }
        }
    }

    let indexes = {
        let conn = state.connections.lock().await;
        match conn.get_table_indexes(&db_instance.id, &req.table_name).await {
            Ok(idx) => idx.into_iter().map(|i| IndexSchema {
                name: i.name,
                columns: i.columns,
                index_type: i.index_type,
                is_unique: i.is_unique,
                is_primary: i.is_primary,
            }).collect(),
            Err(e) => {
                return connect_response(GetTableIndexesResponse {
                    success: false,
                    indexes: vec![],
                    error: Some(format!("Failed to fetch indexes: {}", e)),
                });
            }
        }
    };

    connect_response(GetTableIndexesResponse {
        success: true,
        indexes,
        error: None,
    })
}

/// POST /bennett.v1.SchemaService/GetTableConstraints
pub async fn get_table_constraints(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    let req: GetTableConstraintsRequest = match parse_connect_request(&body.to_string()) {
        Ok(r) => r,
        Err(resp) => return resp,
    };

    let validated = match validate_share_request(&state, &req.share_code, &req.token).await {
        Ok(v) => v,
        Err(resp) => return resp,
    };

    let db_instance = {
        let dbs = state.databases.lock().unwrap();
        dbs.iter().find(|d| d.id == validated.db_id).cloned()
    };

    let db_instance = match db_instance {
        Some(d) => d,
        None => return connect_error("not_found", "Database not available"),
    };

    {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&db_instance.id) {
            if let Err(e) = conn.connect(&db_instance).await {
                return connect_error("unavailable", &format!("Connection failed: {}", e));
            }
        }
    }

    let constraints = {
        let conn = state.connections.lock().await;
        match conn.get_table_constraints(&db_instance.id, &req.table_name).await {
            Ok(c) => c.into_iter().map(|c| ConstraintSchema {
                name: c.name,
                constraint_type: c.constraint_type,
                columns: c.columns,
                definition: c.definition,
            }).collect(),
            Err(e) => {
                return connect_response(GetTableConstraintsResponse {
                    success: false,
                    constraints: vec![],
                    error: Some(format!("Failed to fetch constraints: {}", e)),
                });
            }
        }
    };

    connect_response(GetTableConstraintsResponse {
        success: true,
        constraints,
        error: None,
    })
}

/// TODO: Phase 2 - Implement StreamSchemaUpdates for real-time autocomplete (WebSocket/polling infrastructure)
/// TODO: Phase 3 - Implement column-level permission filtering (filter schema columns based on share config)
/// TODO: Phase 3 - Implement schema caching with TTL (add SchemaCache to AppState, invalidate on DDL)
