use axum::{
    extract::{Path, State, Json},
    http::StatusCode,
};
use tracing::info;

use crate::AppState;
use crate::models::database::{
    DatabaseInstance, DatabaseStatus, CreateDatabaseRequest, UpdateDatabaseRequest, ApiResponse,
};

pub async fn health_check() -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::success(serde_json::json!({
        "status": "ok",
        "version": "0.1.0",
        "engine": "bennett-engine"
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
) -> Json<ApiResponse<DatabaseInstance>> {
    let mut db = state.databases.lock().unwrap();
    
    if db.iter().any(|d| d.name == req.name) {
        return Json(ApiResponse::error(format!("Database '{}' already exists", req.name)));
    }

    let id = format!("{}", db.len() + 1);
    let port = 5432 + db.len() as u16 + 1;
    
    let instance = DatabaseInstance {
        id: id.clone(),
        name: req.name.clone(),
        db_type: req.db_type.clone(),
        version: req.version.clone(),
        status: DatabaseStatus::Starting,
        port,
        size: "0 MB".to_string(),
        created_at: chrono::Local::now().format("%Y-%m-%d").to_string(),
        container_id: Some(format!("{}-{}-{}", req.db_type, req.version, req.name)),
    };

    info!("Creating database {} (type: {}, version: {})", req.name, req.db_type, req.version);
    
    db.push(instance.clone());
    
    let databases = state.databases.clone();
    let instance_id = id.clone();
    tokio::spawn(async move {
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
        let mut db = databases.lock().unwrap();
        if let Some(d) = db.iter_mut().find(|d| d.id == instance_id) {
            d.status = DatabaseStatus::Running;
            d.size = "128 MB".to_string();
            info!("Database {} is now running", instance_id);
        }
    });

    Json(ApiResponse::success(instance))
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
) -> Json<ApiResponse<serde_json::Value>> {
    let mut db = state.databases.lock().unwrap();
    
    if let Some(pos) = db.iter().position(|d| d.id == id) {
        let name = db[pos].name.clone();
        db.remove(pos);
        info!("Deleted database {} ({})", id, name);
        Json(ApiResponse::success(serde_json::json!({ "deleted": true, "id": id })))
    } else {
        Json(ApiResponse::error(format!("Database {} not found", id)))
    }
}

pub async fn start_database(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Json<ApiResponse<DatabaseInstance>> {
    let mut db = state.databases.lock().unwrap();
    
    if let Some(instance) = db.iter_mut().find(|d| d.id == id) {
        if instance.status == DatabaseStatus::Running {
            return Json(ApiResponse::error(format!("Database {} is already running", id)));
        }
        
        instance.status = DatabaseStatus::Starting;
        let name = instance.name.clone();
        drop(db);

        let databases = state.databases.clone();
        let instance_id = id.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            let mut db = databases.lock().unwrap();
            if let Some(d) = db.iter_mut().find(|d| d.id == instance_id) {
                d.status = DatabaseStatus::Running;
                info!("Started database {}", name);
            }
        });

        let db = state.databases.lock().unwrap();
        let instance = db.iter().find(|d| d.id == id).unwrap().clone();
        Json(ApiResponse::success(instance))
    } else {
        Json(ApiResponse::error(format!("Database {} not found", id)))
    }
}

pub async fn stop_database(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Json<ApiResponse<DatabaseInstance>> {
    let mut db = state.databases.lock().unwrap();
    
    if let Some(instance) = db.iter_mut().find(|d| d.id == id) {
        if instance.status == DatabaseStatus::Stopped {
            return Json(ApiResponse::error(format!("Database {} is already stopped", id)));
        }
        
        instance.status = DatabaseStatus::Stopped;
        info!("Stopped database {}", instance.name);
        Json(ApiResponse::success(instance.clone()))
    } else {
        Json(ApiResponse::error(format!("Database {} not found", id)))
    }
}
