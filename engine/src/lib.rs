pub mod models;
pub mod api;

use std::sync::{Arc, Mutex};
use models::database::DatabaseInstance;

#[derive(Clone)]
pub struct AppState {
    pub databases: Arc<Mutex<Vec<DatabaseInstance>>>,
}