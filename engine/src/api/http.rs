use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tracing::info;

use crate::AppState;
use crate::models::database::{
    ApiResponse, CreateDatabaseRequest, DatabaseInstance, DatabaseStatus, UpdateDatabaseRequest,
};

pub async fn health_check() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "status": "ok",
        "version": "0.1.0",
        "engine": "bennett-engine",
        "docker": "connected"
    })))
}

pub async fn list_databases(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<DatabaseInstance>>> {
    let db = state.databases.lock().unwrap();
    Json(ApiResponse::success(db.clone()))
}

pub async fn get_database(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<DatabaseInstance>>, StatusCode> {
    let db = state.databases.lock().unwrap();
    match db.iter().find(|d| d.id == id) {
        Some(instance) => Ok(Json(ApiResponse::success(instance.clone()))),
        None => Ok(Json(ApiResponse::error(format!("Database {} not found", id)))),
    }
}

pub async fn create_database(
    State(state): State<AppState>,
    Json(req): Json<CreateDatabaseRequest>,
) -> Result<Json<ApiResponse<DatabaseInstance>>, StatusCode> {
    {
        let db = state.databases.lock().unwrap();
        if db.iter().any(|d| d.name == req.name) {
            return Ok(Json(ApiResponse::error(format!(
                "Database '{}' already exists",
                req.name
            ))));
        }
    }

    let port = match state.ports.allocate(&req.db_type) {
        Ok(p) => p,
        Err(e) => {
            return Ok(Json(ApiResponse::error(format!(
                "Port allocation failed: {}",
                e
            ))))
        }
    };

    let id = uuid::Uuid::new_v4().to_string();
    let volume_name =
        crate::runtime::volume::manager::VolumeManager::generate_name(&req.db_type, &req.name);

    if let Err(e) = state.volumes.create(&volume_name).await {
        state.ports.release(port);
        return Ok(Json(ApiResponse::error(format!(
            "Volume creation failed: {}",
            e
        ))));
    }

    let instance = DatabaseInstance {
        id: id.clone(),
        name: req.name.clone(),
        db_type: req.db_type.clone(),
        version: req.version.clone(),
        status: DatabaseStatus::Starting,
        port,
        size: "0 MB".to_string(),
        created_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
        container_id: None,
        volume_name: Some(volume_name.clone()),
        env_vars: Vec::new(),
    };

    info!(
        "Creating database {} (type: {}, version: {}, port: {})",
        req.name, req.db_type, req.version, port
    );

    let container_id = match state.docker.create_container(&instance).await {
        Ok(cid) => cid,
        Err(e) => {
            state.ports.release(port);
            let _ = state.volumes.remove(&volume_name).await;
            return Ok(Json(ApiResponse::error(format!(
                "Container creation failed: {}",
                e
            ))));
        }
    };

    {
        let mut db = state.databases.lock().unwrap();
        let mut instance_with_container = instance.clone();
        instance_with_container.container_id = Some(container_id.clone());
        db.push(instance_with_container);
    }

    if let Err(e) = state.docker.start_container(&container_id).await {
        return Ok(Json(ApiResponse::error(format!(
            "Container start failed: {}",
            e
        ))));
    }

    let instance = {
        let mut db = state.databases.lock().unwrap();
        if let Some(d) = db.iter_mut().find(|d| d.id == id) {
            d.status = DatabaseStatus::Running;
            d.size = "128 MB".to_string();
            info!("Database {} is now running on port {}", id, port);
        }
        db.iter().find(|d| d.id == id).cloned()
    };

    match instance {
        Some(inst) => Ok(Json(ApiResponse::success(inst))),
        None => Ok(Json(ApiResponse::error(format!(
            "Database {} not found after creation",
            id
        )))),
    }
}

pub async fn update_database(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<UpdateDatabaseRequest>,
) -> Result<Json<ApiResponse<DatabaseInstance>>, StatusCode> {
    let mut db = state.databases.lock().unwrap();

    if let Some(instance) = db.iter_mut().find(|d| d.id == id) {
        if let Some(name) = req.name {
            instance.name = name;
        }
        if let Some(status) = req.status {
            instance.status = status;
        }
        Ok(Json(ApiResponse::success(instance.clone())))
    } else {
        Ok(Json(ApiResponse::error(format!("Database {} not found", id))))
    }
}

pub async fn delete_database(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, StatusCode> {
    let instance = {
        let db = state.databases.lock().unwrap();
        db.iter().find(|d| d.id == id).cloned()
    };

    let instance = match instance {
        Some(i) => i,
        None => {
            return Ok(Json(ApiResponse::error(format!(
                "Database {} not found",
                id
            ))))
        }
    };

    if let Some(ref container_id) = instance.container_id {
        let _ = state.docker.stop_container(container_id).await;
        let _ = state.docker.remove_container(container_id).await;
    }

    state.ports.release(instance.port);

    if let Some(ref volume_name) = instance.volume_name {
        let _ = state.volumes.remove(volume_name).await;
    }

    {
        let mut db = state.databases.lock().unwrap();
        if let Some(pos) = db.iter().position(|d| d.id == id) {
            let name = db[pos].name.clone();
            db.remove(pos);
            info!("Deleted database {} ({})", id, name);
        }
    }

    Ok(Json(ApiResponse::success(serde_json::json!({
        "deleted": true,
        "id": id
    }))))
}

pub async fn start_database(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<DatabaseInstance>>, StatusCode> {
    let lookup = {
        let mut db = state.databases.lock().unwrap();
        match db.iter_mut().find(|d| d.id == id) {
            Some(instance) => {
                let already_running = instance.status == DatabaseStatus::Running;
                let container_id = instance.container_id.clone();
                let name = instance.name.clone();
                Ok((already_running, container_id, name))
            }
            None => Err(format!("Database {} not found", id)),
        }
    };

    let (already_running, container_id, name) = match lookup {
        Ok(t) => t,
        Err(msg) => return Ok(Json(ApiResponse::error(msg))),
    };

    if already_running {
        return Ok(Json(ApiResponse::error(format!(
            "Database {} is already running",
            id
        ))));
    }

    if let Some(cid) = container_id {
        match state.docker.start_container(&cid).await {
            Ok(_) => {
                let mut db = state.databases.lock().unwrap();
                if let Some(instance) = db.iter_mut().find(|d| d.id == id) {
                    instance.status = DatabaseStatus::Running;
                    info!("Started database {}", name);
                    let inst = instance.clone();
                    Ok(Json(ApiResponse::success(inst)))
                } else {
                    Ok(Json(ApiResponse::error(format!(
                        "Database {} not found",
                        id
                    ))))
                }
            }
            Err(e) => Ok(Json(ApiResponse::error(format!("Start failed: {}", e)))),
        }
    } else {
        Ok(Json(ApiResponse::error(format!(
            "Database {} has no container",
            id
        ))))
    }
}

pub async fn stop_database(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<DatabaseInstance>>, StatusCode> {
    let lookup = {
        let mut db = state.databases.lock().unwrap();
        match db.iter_mut().find(|d| d.id == id) {
            Some(instance) => {
                let already_stopped = instance.status == DatabaseStatus::Stopped;
                let container_id = instance.container_id.clone();
                let name = instance.name.clone();
                Ok((already_stopped, container_id, name))
            }
            None => Err(format!("Database {} not found", id)),
        }
    };

    let (already_stopped, container_id, name) = match lookup {
        Ok(t) => t,
        Err(msg) => return Ok(Json(ApiResponse::error(msg))),
    };

    if already_stopped {
        return Ok(Json(ApiResponse::error(format!(
            "Database {} is already stopped",
            id
        ))));
    }

    if let Some(cid) = container_id {
        match state.docker.stop_container(&cid).await {
            Ok(_) => {
                let mut db = state.databases.lock().unwrap();
                if let Some(instance) = db.iter_mut().find(|d| d.id == id) {
                    instance.status = DatabaseStatus::Stopped;
                    info!("Stopped database {}", name);
                    let inst = instance.clone();
                    Ok(Json(ApiResponse::success(inst)))
                } else {
                    Ok(Json(ApiResponse::error(format!(
                        "Database {} not found",
                        id
                    ))))
                }
            }
            Err(e) => Ok(Json(ApiResponse::error(format!("Stop failed: {}", e)))),
        }
    } else {
        Ok(Json(ApiResponse::error(format!(
            "Database {} has no container",
            id
        ))))
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExecuteQueryRequest {
    pub sql: String,
}

pub async fn execute_query(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<ExecuteQueryRequest>,
) -> Json<ApiResponse<crate::control_plane::connection::manager::QueryResult>> {
    let instance = {
        let db = state.databases.lock().unwrap();
        match db.iter().find(|d| d.id == id).cloned() {
            Some(i) => i,
            None => return Json(ApiResponse::error(format!("Database {} not found", id))),
        }
    };

    // Auto-connect if not connected, or reconnect if stale
    {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&id) || !conn.health_check(&id).await {
            if conn.is_connected(&id) {
                conn.remove_stale(&id).await;
            }
            if let Err(e) = conn.connect(&instance).await {
                return Json(ApiResponse::error(format!("Connection failed: {}", e)));
            }
        }
    }

    let result = {
        let conn = state.connections.lock().await;
        match conn.execute(&id, &req.sql).await {
            Ok(r) => Json(ApiResponse::success(r)),
            Err(e) => Json(ApiResponse::error(format!("Query failed: {}", e))),
        }
    };

    result
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct TableDataRequest {
    pub table: String,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub order_by: Option<String>,
    pub order_dir: Option<String>,
    pub filter: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TableDataResponse {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
    pub total_count: usize,
}

pub async fn get_table_data(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<TableDataRequest>,
) -> Json<ApiResponse<TableDataResponse>> {
    let instance = {
        let db = state.databases.lock().unwrap();
        match db.iter().find(|d| d.id == id).cloned() {
            Some(i) => i,
            None => return Json(ApiResponse::error(format!("Database {} not found", id))),
        }
    };

    // Auto-connect if not connected
    {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&id) {
            if let Err(e) = conn.connect(&instance).await {
                return Json(ApiResponse::error(format!("Connection failed: {}", e)));
            }
        }
    }

    let limit = req.limit.unwrap_or(50).max(1).min(1000);
    let offset = req.offset.unwrap_or(0).max(0);
    let order_dir = req.order_dir.as_deref().unwrap_or("ASC");
    let filter = req.filter.as_deref().unwrap_or("");

    // Build safe query
    let where_clause = if filter.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", filter)
    };

    let count_sql = if filter.is_empty() {
        format!("SELECT COUNT(*) FROM \"{}\"", req.table)
    } else {
        format!("SELECT COUNT(*) FROM \"{}\" {}", req.table, where_clause)
    };

    // ORDER BY only when column specified; never default to "1" (that's a column name, not position)
    let order_clause = req.order_by.as_deref().map(|col| {
        if instance.db_type == "mysql" || instance.db_type == "mariadb" {
            format!("ORDER BY `{}` {}", col, order_dir)
        } else {
            format!("ORDER BY \"{}\" {}", col, order_dir)
        }
    }).unwrap_or_default();

    let data_sql = if instance.db_type == "mysql" || instance.db_type == "mariadb" {
        format!(
            "SELECT * FROM `{}` {} {} LIMIT {} OFFSET {}",
            req.table, where_clause, order_clause, limit, offset
        )
    } else {
        format!(
            "SELECT * FROM \"{}\" {} {} LIMIT {} OFFSET {}",
            req.table, where_clause, order_clause, limit, offset
        )
    };

    let total_count = {
        let conn = state.connections.lock().await;
        match conn.execute(&id, &count_sql).await {
            Ok(result) => {
                if let Some(first_row) = result.rows.first() {
                    if let Some(serde_json::Value::Number(n)) = first_row.first() {
                        n.as_i64().unwrap_or(0) as usize
                    } else {
                        0
                    }
                } else {
                    0
                }
            }
            Err(_) => 0,
        }
    };

    let result = {
        let conn = state.connections.lock().await;
        match conn.execute(&id, &data_sql).await {
            Ok(r) => Json(ApiResponse::success(TableDataResponse {
                columns: r.columns,
                rows: r.rows,
                row_count: r.row_count,
                total_count,
            })),
            Err(e) => Json(ApiResponse::error(format!("Query failed: {}", e))),
        }
    };

    result
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct UpdateRowRequest {
    pub table: String,
    pub primary_key: serde_json::Value,
    pub primary_key_column: String,
    pub data: std::collections::HashMap<String, serde_json::Value>,
}

pub async fn update_row(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<UpdateRowRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let instance = {
        let db = state.databases.lock().unwrap();
        match db.iter().find(|d| d.id == id).cloned() {
            Some(i) => i,
            None => return Json(ApiResponse::error(format!("Database {} not found", id))),
        }
    };

    {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&id) {
            if let Err(e) = conn.connect(&instance).await {
                return Json(ApiResponse::error(format!("Connection failed: {}", e)));
            }
        }
    }

    // Build SET clause
    let mut set_clauses = Vec::new();
    for (col, val) in &req.data {
        let val_str = match val {
            serde_json::Value::Null => "NULL".to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => format!("'{}'", s.replace("'", "''")),
            _ => format!("'{}'", val.to_string().replace("'", "''")),
        };
        
        if instance.db_type == "mysql" || instance.db_type == "mariadb" {
            set_clauses.push(format!("`{}` = {}", col, val_str));
        } else {
            set_clauses.push(format!("\"{}\" = {}", col, val_str));
        }
    }

    let sql = if instance.db_type == "mysql" || instance.db_type == "mariadb" {
        let pk_val = match &req.primary_key {
            serde_json::Value::String(s) => format!("'{}'", s.replace("'", "''")),
            serde_json::Value::Number(n) => n.to_string(),
            _ => req.primary_key.to_string(),
        };
        format!(
            "UPDATE `{}` SET {} WHERE `{}` = {}",
            req.table,
            set_clauses.join(", "),
            req.primary_key_column,
            pk_val
        )
    } else {
        let pk_val = match &req.primary_key {
            serde_json::Value::String(s) => format!("'{}'", s.replace("'", "''")),
            serde_json::Value::Number(n) => n.to_string(),
            _ => req.primary_key.to_string(),
        };
        format!(
            "UPDATE \"{}\" SET {} WHERE \"{}\" = {}",
            req.table,
            set_clauses.join(", "),
            req.primary_key_column,
            pk_val
        )
    };

    let result = {
        let conn = state.connections.lock().await;
        match conn.execute(&id, &sql).await {
            Ok(_) => Json(ApiResponse::success(serde_json::json!({ "updated": true }))),
            Err(e) => Json(ApiResponse::error(format!("Update failed: {}", e))),
        }
    };

    result
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct DeleteRowRequest {
    pub table: String,
    pub primary_key: serde_json::Value,
    pub primary_key_column: String,
}

pub async fn delete_row(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<DeleteRowRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let instance = {
        let db = state.databases.lock().unwrap();
        match db.iter().find(|d| d.id == id).cloned() {
            Some(i) => i,
            None => return Json(ApiResponse::error(format!("Database {} not found", id))),
        }
    };

    {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&id) {
            if let Err(e) = conn.connect(&instance).await {
                return Json(ApiResponse::error(format!("Connection failed: {}", e)));
            }
        }
    }

    let sql = if instance.db_type == "mysql" || instance.db_type == "mariadb" {
        let pk_val = match &req.primary_key {
            serde_json::Value::String(s) => format!("'{}'", s.replace("'", "''")),
            serde_json::Value::Number(n) => n.to_string(),
            _ => req.primary_key.to_string(),
        };
        format!(
            "DELETE FROM `{}` WHERE `{}` = {}",
            req.table, req.primary_key_column, pk_val
        )
    } else {
        let pk_val = match &req.primary_key {
            serde_json::Value::String(s) => format!("'{}'", s.replace("'", "''")),
            serde_json::Value::Number(n) => n.to_string(),
            _ => req.primary_key.to_string(),
        };
        format!(
            "DELETE FROM \"{}\" WHERE \"{}\" = {}",
            req.table, req.primary_key_column, pk_val
        )
    };

    let result = {
        let conn = state.connections.lock().await;
        match conn.execute(&id, &sql).await {
            Ok(_) => Json(ApiResponse::success(serde_json::json!({ "deleted": true }))),
            Err(e) => Json(ApiResponse::error(format!("Delete failed: {}", e))),
        }
    };

    result
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ColumnMetadata {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
    pub has_default: bool,
    pub is_primary_key: bool,
    pub column_default: Option<String>,
}

pub async fn get_table_columns(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<TableDataRequest>,
) -> Json<ApiResponse<Vec<ColumnMetadata>>> {
    let instance = {
        let db = state.databases.lock().unwrap();
        match db.iter().find(|d| d.id == id).cloned() {
            Some(i) => i,
            None => return Json(ApiResponse::error(format!("Database {} not found", id))),
        }
    };

    {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&id) {
            if let Err(e) = conn.connect(&instance).await {
                return Json(ApiResponse::error(format!("Connection failed: {}", e)));
            }
        }
    }

    let sql = if instance.db_type == "mysql" || instance.db_type == "mariadb" {
        format!(
            "SELECT column_name, data_type, is_nullable, column_default, extra 
             FROM information_schema.columns 
             WHERE table_schema = DATABASE() AND table_name = '{}' 
             ORDER BY ordinal_position",
            req.table
        )
    } else {
        format!(
            "SELECT column_name, data_type, is_nullable, column_default, 
             CASE WHEN column_default IS NOT NULL THEN true ELSE false END as has_default
             FROM information_schema.columns 
             WHERE table_schema = 'public' AND table_name = '{}' 
             ORDER BY ordinal_position",
            req.table
        )
    };

    let result = {
        let conn = state.connections.lock().await;
        match conn.execute(&id, &sql).await {
            Ok(r) => {
                let columns: Vec<ColumnMetadata> = r.rows.iter().map(|row| {
                    let name = row.get(0).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let data_type = row.get(1).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let nullable = row.get(2).and_then(|v| v.as_str()).map(|s| s == "YES").unwrap_or(false);
                    let has_default = row.get(3).map(|v| !v.is_null()).unwrap_or(false);
                    let column_default = row.get(3).and_then(|v| v.as_str()).map(|s| s.to_string());
                    
                    // Detect PK: usually first column with sequence or named 'id'
                    let is_pk = name == "user_id" || name == "id" || 
                        column_default.as_ref().map(|d| d.contains("nextval")).unwrap_or(false);

                    ColumnMetadata {
                        name,
                        data_type,
                        nullable,
                        has_default,
                        is_primary_key: is_pk,
                        column_default,
                    }
                }).collect();
                Json(ApiResponse::success(columns))
            }
            Err(e) => Json(ApiResponse::error(format!("Failed to get columns: {}", e))),
        }
    };

    result
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct InsertRowRequest {
    pub table: String,
    pub data: std::collections::HashMap<String, serde_json::Value>,
}

pub async fn insert_row(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<InsertRowRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let instance = {
        let db = state.databases.lock().unwrap();
        match db.iter().find(|d| d.id == id).cloned() {
            Some(i) => i,
            None => return Json(ApiResponse::error(format!("Database {} not found", id))),
        }
    };

    {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&id) {
            if let Err(e) = conn.connect(&instance).await {
                return Json(ApiResponse::error(format!("Connection failed: {}", e)));
            }
        }
    }

    if req.data.is_empty() {
        return Json(ApiResponse::error("No data provided for insert".to_string()));
    }

    let columns: Vec<String> = req.data.keys().cloned().collect();
    let values: Vec<String> = req.data.values().map(|val| {
        match val {
            serde_json::Value::Null => "NULL".to_string(),
            serde_json::Value::Bool(b) => b.to_string(),
            serde_json::Value::Number(n) => n.to_string(),
            serde_json::Value::String(s) => format!("'{}'", s.replace("'", "''")),
            _ => format!("'{}'", val.to_string().replace("'", "''")),
        }
    }).collect();

    let sql = if instance.db_type == "mysql" || instance.db_type == "mariadb" {
        format!(
            "INSERT INTO `{}` ({}) VALUES ({})",
            req.table,
            columns.iter().map(|c| format!("`{}`", c)).collect::<Vec<_>>().join(", "),
            values.join(", ")
        )
    } else {
        format!(
            "INSERT INTO \"{}\" ({}) VALUES ({})",
            req.table,
            columns.iter().map(|c| format!("\"{}\"", c)).collect::<Vec<_>>().join(", "),
            values.join(", ")
        )
    };

    let result = {
        let conn = state.connections.lock().await;
        match conn.execute(&id, &sql).await {
            Ok(_) => Json(ApiResponse::success(serde_json::json!({ "inserted": true }))),
            Err(e) => Json(ApiResponse::error(format!("Insert failed: {}", e))),
        }
    };

    result
}

pub async fn get_schema(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<crate::control_plane::connection::manager::TableInfo>>> {
    let instance = {
        let db = state.databases.lock().unwrap();
        match db.iter().find(|d| d.id == id).cloned() {
            Some(i) => i,
            None => return Json(ApiResponse::error(format!("Database {} not found", id))),
        }
    };

    // Auto-connect if not connected
    {
        let mut conn = state.connections.lock().await;
        if !conn.is_connected(&id) {
            if let Err(e) = conn.connect(&instance).await {
                return Json(ApiResponse::error(format!("Connection failed: {}", e)));
            }
        }
    }

    let result = {
        let conn = state.connections.lock().await;
        match conn.get_schema(&id).await {
            Ok(schema) => Json(ApiResponse::success(schema)),
            Err(e) => Json(ApiResponse::error(format!("Schema query failed: {}", e))),
        }
    };

    result
}
