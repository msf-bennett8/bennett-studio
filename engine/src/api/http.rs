use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tracing::info;

use crate::AppState;
use crate::models::database::{
    ApiResponse, CreateDatabaseRequest, DatabaseInstance, DatabaseStatus, UpdateDatabaseRequest,
    DatabaseSource, UnlockDatabaseRequest, DatabaseStatusResponse, DatabaseCredentials,
};
use crate::runtime::discovery::scanner::LocalScanner;

// ============================================================================
// Input Validation Helpers
// ============================================================================

fn validate_db_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Database name cannot be empty".to_string());
    }
    if name.len() > 64 {
        return Err("Database name too long (max 64 chars)".to_string());
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        return Err("Database name must be alphanumeric, underscore, or hyphen only".to_string());
    }
    Ok(())
}

fn validate_filter(filter: &str) -> Result<(), String> {
    let forbidden = [";", "--", "/*", "*/", "DROP", "DELETE", "UPDATE", "INSERT", "EXEC", "UNION"];
    let upper = filter.to_uppercase();
    for word in &forbidden {
        if upper.contains(word) {
            return Err(format!("Filter contains forbidden keyword: {}", word));
        }
    }
    // Count quotes to prevent injection
    let single_quotes = filter.chars().filter(|&c| c == '\'').count();
    let double_quotes = filter.chars().filter(|&c| c == '"').count();
    if single_quotes % 2 != 0 || double_quotes % 2 != 0 {
        return Err("Unmatched quotes in filter".to_string());
    }
    Ok(())
}

fn validate_sql(sql: &str) -> Result<(), String> {
    // Block multi-statement queries
    if sql.contains(';') {
        return Err("Multiple statements are not allowed".to_string());
    }
    // Block dangerous keywords at statement level
    let upper = sql.to_uppercase();
    let forbidden_starts = ["DROP", "TRUNCATE", "ALTER SYSTEM", "COPY", "\\COPY"];
    for word in &forbidden_starts {
        if upper.trim_start().starts_with(word) {
            return Err(format!("Statement type not allowed: {}", word));
        }
    }
    Ok(())
}

// ============================================================================
// Health
// ============================================================================

pub async fn health_check() -> Json<ApiResponse<serde_json::Value>> {
    crate::api::health::simple_health_check().await
}

// ============================================================================
// Database CRUD
// ============================================================================

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
    // Validate name
    if let Err(e) = validate_db_name(&req.name) {
        return Ok(Json(ApiResponse::error(e)));
    }

    {
        let db = state.databases.lock().unwrap();
        if db.iter().any(|d| d.name == req.name) {
            return Ok(Json(ApiResponse::error(format!(
                "Database '{}' already exists",
                req.name
            ))));
        }
    }

    // Skip port allocation for SQLite
    let port = if req.db_type == "sqlite" {
        0
    } else {
        match state.ports.allocate(&req.db_type) {
            Ok(p) => p,
            Err(e) => {
                return Ok(Json(ApiResponse::error(format!(
                    "Port allocation failed: {}",
                    e
                ))))
            }
        }
    };

    let id = uuid::Uuid::new_v4().to_string();
    let volume_name =
        crate::runtime::volume::manager::VolumeManager::generate_name(&req.db_type, &req.name);

    if req.db_type != "sqlite" {
        if let Err(e) = state.volumes.create(&volume_name).await {
            if port != 0 {
                state.ports.release(port);
            }
            return Ok(Json(ApiResponse::error(format!(
                "Volume creation failed: {}",
                e
            ))));
        }
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
        volume_name: if req.db_type == "sqlite" { None } else { Some(volume_name.clone()) },
        env_vars: Vec::new(),
        source: DatabaseSource::Bennett,
        is_discovered: false,
        credentials: None,
        is_unlocked: false,
    };

    info!(
        "Creating database {} (type: {}, version: {}, port: {})",
        req.name, req.db_type, req.version, port
    );

    let container_id = if req.db_type == "sqlite" {
        None
    } else {
        match state.docker.create_container(&instance).await {
            Ok(cid) => Some(cid),
            Err(e) => {
                if port != 0 {
                    state.ports.release(port);
                }
                if req.db_type != "sqlite" {
                    let _ = state.volumes.remove(&volume_name).await;
                }
                return Ok(Json(ApiResponse::error(format!(
                    "Container creation failed: {}",
                    e
                ))));
            }
        }
    };

    {
        let mut db = state.databases.lock().unwrap();
        let mut instance_with_container = instance.clone();
        instance_with_container.container_id = container_id.clone();
        db.push(instance_with_container);
    }

    if let Some(ref cid) = container_id {
        if let Err(e) = state.docker.start_container(cid).await {
            return Ok(Json(ApiResponse::error(format!(
                "Container start failed: {}",
                e
            ))));
        }
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
            if let Err(e) = validate_db_name(&name) {
                return Ok(Json(ApiResponse::error(e)));
            }
            instance.name = name.to_string();
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

    // Prevent deleting local-discovered databases
    if instance.source == DatabaseSource::Local {
        return Ok(Json(ApiResponse::error(
            "Cannot delete a locally-discovered database. Remove it from the list instead.".to_string()
        )));
    }

    if let Some(ref container_id) = instance.container_id {
        let _ = state.docker.stop_container(container_id).await;
        let _ = state.docker.remove_container(container_id).await;
    }

    if instance.port != 0 {
        state.ports.release(instance.port);
    }

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
                Ok((already_running, container_id, name, instance.source.clone()))
            }
            None => Err(format!("Database {} not found", id)),
        }
    };

    let (already_running, container_id, name, source) = match lookup {
        Ok(t) => t,
        Err(msg) => return Ok(Json(ApiResponse::error(msg))),
    };

    if source == DatabaseSource::Local {
        return Ok(Json(ApiResponse::error(
            "Cannot start/stop a locally-discovered database".to_string()
        )));
    }

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
                let source = instance.source.clone();
                Ok((already_stopped, container_id, name, source))
            }
            None => Err(format!("Database {} not found", id)),
        }
    };

    let (already_stopped, container_id, name, source) = match lookup {
        Ok(t) => t,
        Err(msg) => return Ok(Json(ApiResponse::error(msg))),
    };

    if source == DatabaseSource::Local {
        return Ok(Json(ApiResponse::error(
            "Cannot start/stop a locally-discovered database".to_string()
        )));
    }

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

// ============================================================================
// Unlock / Credentials xxxx
// ============================================================================

pub async fn unlock_database(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<UnlockDatabaseRequest>,
) -> Json<ApiResponse<DatabaseStatusResponse>> {
    let instance = {
        let db = state.databases.lock().unwrap();
        match db.iter().find(|d| d.id == id).cloned() {
            Some(i) => i,
            None => return Json(ApiResponse::error(format!("Database {} not found", id))),
        }
    };

    // Test connection with provided credentials
    {
        let mut conn = state.connections.lock().await;
        // Clear any existing stale credentials first
        conn.clear_credentials(&id);
        
        let creds = DatabaseCredentials {
            username: req.username.clone(),
            password: req.password.clone(),
            database: req.database.clone(),
        };
        conn.store_credentials(&id, creds);
        
        // Test the connection
        if conn.is_connected(&id) {
            conn.remove_stale(&id).await;
        }
        
        match conn.connect(&instance).await {
            Ok(_) => {
                // Connection successful — update instance state
                let mut db = state.databases.lock().unwrap();
                if let Some(inst) = db.iter_mut().find(|d| d.id == id) {
                    inst.is_unlocked = true;
                    inst.credentials = Some(DatabaseCredentials {
                        username: req.username,
                        password: req.password,
                        database: req.database,
                    });
                }
                
                Json(ApiResponse::success(DatabaseStatusResponse {
                    id: id.clone(),
                    is_connected: true,
                    is_unlocked: true,
                    has_credentials: true,
                    last_error: None,
                }))
            }
            Err(e) => {
                // Connection failed — clear credentials
                conn.clear_credentials(&id);
                Json(ApiResponse::error(format!("Authentication failed: {}", e)))
            }
        }
    }
}

pub async fn get_database_status(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Json<ApiResponse<DatabaseStatusResponse>> {
    let instance = {
        let db = state.databases.lock().unwrap();
        match db.iter().find(|d| d.id == id).cloned() {
            Some(i) => i,
            None => return Json(ApiResponse::error(format!("Database {} not found", id))),
        }
    };

    let is_connected = {
        let conn = state.connections.lock().await;
        conn.is_connected(&id) && conn.health_check(&id).await
    };

    let has_credentials = instance.credentials.is_some() || {
        let conn = state.connections.lock().await;
        conn.has_credentials(&id)
    };

    Json(ApiResponse::success(DatabaseStatusResponse {
        id: id.clone(),
        is_connected,
        is_unlocked: instance.is_unlocked,
        has_credentials,
        last_error: None,
    }))
}

// ============================================================================
// .env File Scanner for Auto-Suggest
// ============================================================================

use std::path::PathBuf;

pub async fn scan_env_files(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<serde_json::Value>>> {
    let instance = {
        let db = state.databases.lock().unwrap();
        match db.iter().find(|d| d.id == id).cloned() {
            Some(i) => i,
            None => return Json(ApiResponse::error(format!("Database {} not found", id))),
        }
    };

    let mut suggestions = Vec::new();
    
    // Common locations to scan for .env files
    let scan_paths = [
        std::env::var("HOME").map(|h| PathBuf::from(h)).unwrap_or_else(|_| PathBuf::from(".")),
    ];
    
    // Subdirectories to check
    let subdirs = ["oshocks", "oshocks/backend", "backend", ".", ".."];
    
    let home = scan_paths[0].clone();
    
    for subdir in &subdirs {
        let env_path = home.join(subdir).join(".env");
        if env_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&env_path) {
                let mut username = None;
                let mut password = None;
                let mut database = None;
                let mut host = None;
                let mut port = None;
                
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("DB_USERNAME=") || line.starts_with("DB_USER=") {
                        username = line.splitn(2, '=').nth(1).map(|s| s.trim().trim_matches('"').trim_matches('\''));
                    }
                    if line.starts_with("DB_PASSWORD=") || line.starts_with("DB_PASS=") {
                        password = line.splitn(2, '=').nth(1).map(|s| s.trim().trim_matches('"').trim_matches('\''));
                    }
                    if line.starts_with("DB_DATABASE=") || line.starts_with("DB_NAME=") || line.starts_with("DB_DB=") {
                        database = line.splitn(2, '=').nth(1).map(|s| s.trim().trim_matches('"').trim_matches('\''));
                    }
                    if line.starts_with("DB_HOST=") {
                        host = line.splitn(2, '=').nth(1).map(|s| s.trim().trim_matches('"').trim_matches('\''));
                    }
                    if line.starts_with("DB_PORT=") {
                        port = line.splitn(2, '=').nth(1).map(|s| s.trim().trim_matches('"').trim_matches('\''));
                    }
                }
                
                // Only suggest if port matches or host is localhost/127.0.0.1
                let port_matches = port.map(|p| p == instance.port.to_string()).unwrap_or(true);
                let host_matches = host.map(|h| h == "localhost" || h == "127.0.0.1").unwrap_or(true);
                
                if port_matches && host_matches && (username.is_some() || database.is_some()) {
                    suggestions.push(serde_json::json!({
                        "source": env_path.to_string_lossy(),
                        "username": username,
                        "password": password,
                        "database": database,
                        "host": host,
                        "port": port,
                    }));
                }
            }
        }
    }

    Json(ApiResponse::success(suggestions))
}

// ============================================================================
// Discovery
// ============================================================================

pub async fn discover_local_databases(
    State(state): State<AppState>,
) -> Json<ApiResponse<Vec<DatabaseInstance>>> {
    let scanner = LocalScanner::new();
    let existing = {
        let db = state.databases.lock().unwrap();
        db.clone()
    };
    let discovered = scanner.scan(&existing).await;

    let mut db = state.databases.lock().unwrap();
    let mut added = Vec::new();

    for disc in discovered {
        let instance = scanner.to_instance(&disc);
        
        // If this port already has a Bennett DB, mark it as discovered instead of duplicating
        if instance.port != 0 {
            if let Some(existing) = db.iter_mut().find(|d| d.port == instance.port && d.source == DatabaseSource::Bennett) {
                existing.is_discovered = true;
                info!("Marked Bennett database {} on port {} as also discovered locally", existing.name, existing.port);
                added.push(existing.clone());
                continue;
            }
        }
        
        // Check for filesystem-discovered DBs that might match existing by name
        if instance.port == 0 {
            if let Some(existing) = db.iter_mut().find(|d| d.name == instance.name && d.source == DatabaseSource::Bennett) {
                existing.is_discovered = true;
                info!("Marked Bennett database {} (filesystem match) as also discovered", existing.name);
                added.push(existing.clone());
                continue;
            }
        }
        
        // New local-only database
        if !db.iter().any(|d| d.id == instance.id) {
            info!("Adding discovered local database: {} on port {}", instance.name, instance.port);
            db.push(instance.clone());
            added.push(instance);
        }
    }

    Json(ApiResponse::success(added))
}

// ============================================================================
// Query Execution
// ============================================================================

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExecuteQueryRequest {
    pub sql: String,
}

pub async fn execute_query(
    Path(id): Path<String>,
    State(state): State<AppState>,
    Json(req): Json<ExecuteQueryRequest>,
) -> Json<ApiResponse<crate::control_plane::connection::manager::QueryResult>> {
    // Validate SQL
    if let Err(e) = validate_sql(&req.sql) {
        return Json(ApiResponse::error(e));
    }

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

// ============================================================================
// Table Data
// ============================================================================

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

    // Validate filter
    let filter = req.filter.as_deref().unwrap_or("");
    if !filter.is_empty() {
        if let Err(e) = validate_filter(filter) {
            return Json(ApiResponse::error(e));
        }
    }

    let where_clause = if filter.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", filter)
    };

    // Validate table name
    if req.table.is_empty() || req.table.len() > 128 {
        return Json(ApiResponse::error("Invalid table name".to_string()));
    }

    let count_sql = if filter.is_empty() {
        format!("SELECT COUNT(*) FROM \"{}\"", req.table)
    } else {
        format!("SELECT COUNT(*) FROM \"{}\" {}", req.table, where_clause)
    };

    let order_clause = req.order_by.as_deref().filter(|c| !c.is_empty()).map(|col| {
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

// ============================================================================
// Row Operations
// ============================================================================

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

    if req.data.is_empty() {
        return Json(ApiResponse::error("No data provided for update".to_string()));
    }

    // Validate table name
    if req.table.is_empty() || req.table.len() > 128 {
        return Json(ApiResponse::error("Invalid table name".to_string()));
    }

    let mut set_clauses = Vec::new();
    for (col, val) in &req.data {
        if col.is_empty() || col.len() > 128 {
            return Json(ApiResponse::error("Invalid column name".to_string()));
        }
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

    if req.table.is_empty() || req.table.len() > 128 {
        return Json(ApiResponse::error("Invalid table name".to_string()));
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

    if req.table.is_empty() || req.table.len() > 128 {
        return Json(ApiResponse::error("Invalid table name".to_string()));
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

    if req.table.is_empty() || req.table.len() > 128 {
        return Json(ApiResponse::error("Invalid table name".to_string()));
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
