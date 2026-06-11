use sqlx::{Pool, Postgres, MySql, Sqlite};
use std::collections::HashMap;

pub enum DatabasePool {
    Postgres(Pool<Postgres>),
    MySql(Pool<MySql>),
    Sqlite(Pool<Sqlite>),
}

pub struct ConnectionManager {
    #[allow(dead_code)]
    pools: std::sync::Mutex<HashMap<String, DatabasePool>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            pools: std::sync::Mutex::new(HashMap::new()),
        }
    }

    // TODO: Implement connect, disconnect, execute, query in Phase 2
}
