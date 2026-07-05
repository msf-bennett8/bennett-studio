pub mod api;
pub mod audit;
pub mod auth;
pub mod config;
pub mod connect_rpc;
pub mod control_plane;
pub mod errors;
pub mod grpc;
pub mod models;
pub mod plugins;
pub mod rate_limit;
pub mod runtime;
pub mod sharing;
pub mod telemetry;
pub mod utils;
pub mod wasm;

use std::sync::{Arc, Mutex};
use models::database::DatabaseInstance;
use runtime::container::docker::DockerRuntime;
use runtime::port::allocator::PortAllocator;
use runtime::volume::manager::VolumeManager;
use control_plane::connection::manager::ConnectionManager;
use sharing::share_store::ShareStore;
use auth::share_token::ShareTokenManager;
use audit::AuditService;
use rate_limit::RateLimitService;
use control_plane::query::cache::QueryCache;
use control_plane::notes_store::NotesStore;
use api::websocket_buffer::WsMessageBuffer;

#[derive(Clone)]
pub struct AppState {
    pub databases: Arc<Mutex<Vec<DatabaseInstance>>>,
    pub docker: Arc<DockerRuntime>,
    pub ports: Arc<PortAllocator>,
    pub volumes: Arc<VolumeManager>,
    pub connections: Arc<tokio::sync::Mutex<ConnectionManager>>,
    pub share_store: Arc<ShareStore>,
    pub token_manager: Arc<tokio::sync::RwLock<ShareTokenManager>>,
    pub audit_service: Option<Arc<AuditService>>,
    pub rate_limiter: Arc<RateLimitService>,
    pub query_cache: Arc<QueryCache>,
    pub ws_buffer: Arc<WsMessageBuffer>,
    pub notes_store: Arc<tokio::sync::RwLock<NotesStore>>,
}

impl AppState {
    pub async fn new() -> Result<Self, crate::runtime::container::docker::DockerError> {
        let home = dirs::home_dir()
            .ok_or_else(|| crate::runtime::container::docker::DockerError::Other("No home dir".to_string()))?;
        let data_dir = home.join(".bennett").join("data");
        std::fs::create_dir_all(&data_dir).expect("Failed to create .bennett/data directory");

        // sqlx SQLite URL format: use percent-encoding for spaces in path
        let db_file = data_dir.join("shares.db");
        let audit_file = data_dir.join("audit.db");
        
        // Percent-encode spaces and special characters for URL format
        let db_path_encoded = db_file.to_string_lossy().replace(' ', "%20");
        let audit_path_encoded = audit_file.to_string_lossy().replace(' ', "%20");
        
        let db_path = format!("sqlite:{}", db_path_encoded);
        let audit_path = format!("sqlite:{}", audit_path_encoded);
        
        let share_store = ShareStore::new(&db_path).await
            .map_err(|e| crate::runtime::container::docker::DockerError::Other(e.to_string()))?;
        
        let token_manager = ShareTokenManager::new().await
            .map_err(|e| crate::runtime::container::docker::DockerError::Other(e.to_string()))?;
        
        // Initialize audit service (optional - don't fail if it doesn't work)
        let audit_service = match AuditService::new(&audit_path).await {
            Ok(s) => Some(s),
            Err(e) => {
                tracing::warn!("Audit service failed to initialize: {} - continuing without audit logging", e);
                None
            }
        };
        
                let rate_limiter = Arc::new(RateLimitService::new());

        // Initialize notes store — reuse the same SQLite DB as shares to avoid file/path issues
        let notes_store = Arc::new(tokio::sync::RwLock::new(
            NotesStore::new(&db_path).await
                .map_err(|e| crate::runtime::container::docker::DockerError::Other(format!("Notes store init failed: {}", e)))?
        ));

        let query_cache = Arc::new(QueryCache::new());
        
        Ok(Self {
            databases: Arc::new(Mutex::new(Vec::new())),
            docker: Arc::new(DockerRuntime::new()?),
            ports: Arc::new(PortAllocator::new()),
            volumes: Arc::new(VolumeManager::new()?),
            connections: Arc::new(tokio::sync::Mutex::new(ConnectionManager::new())),
            share_store,
            token_manager,
            audit_service,
            rate_limiter,
            query_cache: Arc::new(QueryCache::new()),
            ws_buffer: Arc::new(WsMessageBuffer::new()),
            notes_store,
        })
    }
}
