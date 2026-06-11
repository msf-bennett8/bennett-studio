pub mod api;
pub mod auth;
pub mod config;
pub mod control_plane;
pub mod errors;
pub mod models;
pub mod plugins;
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

#[derive(Clone)]
pub struct AppState {
    pub databases: Arc<Mutex<Vec<DatabaseInstance>>>,
    pub docker: Arc<DockerRuntime>,
    pub ports: Arc<PortAllocator>,
    pub volumes: Arc<VolumeManager>,
    pub connections: Arc<tokio::sync::Mutex<ConnectionManager>>,
}

impl AppState {
    pub fn new() -> Result<Self, crate::runtime::container::docker::DockerError> {
        Ok(Self {
            databases: Arc::new(Mutex::new(Vec::new())),
            docker: Arc::new(DockerRuntime::new()?),
            ports: Arc::new(PortAllocator::new()),
            volumes: Arc::new(VolumeManager::new()?),
            connections: Arc::new(tokio::sync::Mutex::new(ConnectionManager::new())),
        })
    }
}
