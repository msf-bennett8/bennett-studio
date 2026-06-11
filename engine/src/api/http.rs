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
        match conn.execute(&id, &req.sql).await {
            Ok(r) => Json(ApiResponse::success(r)),
            Err(e) => Json(ApiResponse::error(format!("Query failed: {}", e))),
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
