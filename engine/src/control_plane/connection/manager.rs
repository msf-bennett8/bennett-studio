use std::collections::HashMap;
use tracing::info;
use sqlx::{Column, Row};

use crate::models::database::DatabaseInstance;

pub struct ConnectionManager {
    pools: HashMap<String, DatabasePool>,
    credentials: HashMap<String, DatabaseCredentials>,
}

pub use crate::models::database::DatabaseCredentials;

pub enum DatabasePool {
    Postgres(sqlx::PgPool),
    MySql(sqlx::MySqlPool),
    Sqlite(sqlx::SqlitePool),
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
            credentials: HashMap::new(),
        }
    }

    pub fn is_connected(&self, id: &str) -> bool {
        self.pools.contains_key(id)
    }

    pub async fn health_check(&self, id: &str) -> bool {
        let Some(pool) = self.pools.get(id) else {
            return false;
        };
        
        let result = match pool {
            DatabasePool::Postgres(p) => sqlx::query("SELECT 1").fetch_one(p).await.is_ok(),
            DatabasePool::MySql(p) => sqlx::query("SELECT 1").fetch_one(p).await.is_ok(),
            DatabasePool::Sqlite(p) => sqlx::query("SELECT 1").fetch_one(p).await.is_ok(),
        };
        
        if !result {
            tracing::warn!("Health check failed for pool {}", id);
        }
        result
    }

    pub async fn remove_stale(&mut self, id: &str) {
        if let Some(pool) = self.pools.remove(id) {
            match pool {
                DatabasePool::Postgres(p) => p.close().await,
                DatabasePool::MySql(p) => p.close().await,
                DatabasePool::Sqlite(p) => p.close().await,
            }
            tracing::info!("Removed stale pool for {}", id);
        }
    }

    pub async fn connect(&mut self, instance: &DatabaseInstance) -> Result<(), sqlx::Error> {
        if self.pools.contains_key(&instance.id) {
            info!("Connection already exists for {}", instance.id);
            return Ok(());
        }

        let pool = match instance.db_type.as_str() {
            "postgres" | "postgresql" => {
                let url = format!(
                    "postgres://bennett:bennett_secret@localhost:{}/bennett",
                    instance.port
                );
                let pool = sqlx::postgres::PgPoolOptions::new()
                    .max_connections(5)
                    .connect(&url)
                    .await?;
                DatabasePool::Postgres(pool)
            }
            "mysql" | "mariadb" => {
                // For discovered local DBs with port 0 (not running), skip connection
                if instance.port == 0 {
                    return Err(sqlx::Error::Configuration(
                        "Database is not running — start the service first".into(),
                    ));
                }
                // Priority: 1) stored credentials, 2) instance credentials, 3) env_vars, 4) defaults
                let (username, password, database) = if let Some(creds) = self.credentials.get(&instance.id) {
                    (creds.username.clone(), creds.password.clone(), creds.database.clone())
                } else if let Some(creds) = &instance.credentials {
                    (creds.username.clone(), creds.password.clone(), creds.database.clone())
                } else {
                    let username = instance.env_vars.iter().find(|(k, _)| k == "username").map(|(_, v)| v.clone()).unwrap_or_else(|| "bennett".to_string());
                    let password = instance.env_vars.iter().find(|(k, _)| k == "password").map(|(_, v)| v.clone()).unwrap_or_else(|| "bennett_secret".to_string());
                    let database = instance.env_vars.iter().find(|(k, _)| k == "database").map(|(_, v)| v.clone()).unwrap_or_else(|| "bennett".to_string());
                    (username, password, database)
                };
                let url = format!(
                    "mysql://{}:{}@localhost:{}/{}",
                    username, password, instance.port, database
                );
                let pool = sqlx::mysql::MySqlPoolOptions::new()
                    .max_connections(5)
                    .connect(&url)
                    .await?;
                DatabasePool::MySql(pool)
            }
            "sqlite" => {
                let safe_name: String = instance.name.chars()
                    .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                    .collect();
                if safe_name.is_empty() {
                    return Err(sqlx::Error::Configuration(
                        "Invalid SQLite database name".into(),
                    ));
                }
                let path = format!("sqlite://./bennett_{}.db", safe_name);
                let pool = sqlx::sqlite::SqlitePoolOptions::new()
                    .max_connections(1)
                    .connect(&path)
                    .await?;
                DatabasePool::Sqlite(pool)
            }
            _ => {
                return Err(sqlx::Error::Configuration(
                    format!("Unsupported database type: {}", instance.db_type).into(),
                ));
            }
        };

        info!("Connected to {} on port {}", instance.name, instance.port);
        self.pools.insert(instance.id.clone(), pool);
        Ok(())
    }

    pub async fn disconnect(&mut self, id: &str) {
        if let Some(pool) = self.pools.remove(id) {
            match pool {
                DatabasePool::Postgres(p) => p.close().await,
                DatabasePool::MySql(p) => p.close().await,
                DatabasePool::Sqlite(p) => p.close().await,
            }
            info!("Disconnected from {}", id);
        }
    }

    pub fn store_credentials(&mut self, id: &str, creds: DatabaseCredentials) {
        self.credentials.insert(id.to_string(), creds);
    }

    pub fn clear_credentials(&mut self, id: &str) {
        self.credentials.remove(id);
    }

    pub fn has_credentials(&self, id: &str) -> bool {
        self.credentials.contains_key(id)
    }

    pub fn get_credentials(&self, id: &str) -> Option<&DatabaseCredentials> {
        self.credentials.get(id)
    }

    pub async fn execute(
        &self,
        id: &str,
        sql: &str,
    ) -> Result<QueryResult, sqlx::Error> {
        tracing::info!("ConnectionManager.execute: id={}, sql={:?}", id, sql);
        let pool = self.pools.get(id).ok_or_else(|| {
            sqlx::Error::Configuration("Database not connected".into())
        })?;

        match pool {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(sql).fetch_all(pool).await?;
                tracing::info!("Postgres execute: fetched {} rows", rows.len());
                let columns = if let Some(row) = rows.first() {
                    row.columns().iter().map(|c| c.name().to_string()).collect()
                } else {
                    Vec::new()
                };
                let data: Vec<Vec<serde_json::Value>> = rows
                    .iter()
                    .map(|row| {
                        columns
                            .iter()
                            .enumerate()
                            .map(|(i, _)| pg_value_to_json(row, i))
                            .collect()
                    })
                    .collect();
                // Extract last_insert_id from RETURNING clause or pg_last_oid
                let last_insert_id = if sql.trim().to_uppercase().starts_with("INSERT") {
                    // Check if RETURNING clause exists
                    if sql.to_uppercase().contains("RETURNING") {
                        rows.first().and_then(|row| {
                            row.try_get::<String, _>(0).ok()
                                .or_else(|| row.try_get::<i64, _>(0).ok().map(|v| v.to_string()))
                        })
                    } else {
                        // Use pg_last_oid for tables with OID
                        sqlx::query("SELECT pg_last_oid()::text")
                            .fetch_optional(pool)
                            .await
                            .ok()
                            .flatten()
                            .and_then(|r| r.try_get::<String, _>(0).ok())
                    }
                } else {
                    None
                };

                Ok(QueryResult {
                    columns,
                    rows: data,
                    row_count: rows.len(),
                    last_insert_id,
                })
            }
            DatabasePool::MySql(pool) => {
                let rows = sqlx::query(sql).fetch_all(pool).await?;
                let columns = if let Some(row) = rows.first() {
                    row.columns().iter().map(|c| c.name().to_string()).collect()
                } else {
                    Vec::new()
                };
                let data: Vec<Vec<serde_json::Value>> = rows
                    .iter()
                    .map(|row| {
                        columns
                            .iter()
                            .enumerate()
                            .map(|(i, _)| mysql_value_to_json(row, i))
                            .collect()
                    })
                    .collect();
                // Extract last_insert_id for INSERT operations
                let last_insert_id = if sql.trim().to_uppercase().starts_with("INSERT") {
                    sqlx::query("SELECT LAST_INSERT_ID() as id")
                        .fetch_optional(pool)
                        .await
                        .ok()
                        .flatten()
                        .and_then(|r| r.try_get::<i64, _>("id").ok().map(|v| v.to_string()))
                } else {
                    None
                };

                Ok(QueryResult {
                    columns,
                    rows: data,
                    row_count: rows.len(),
                    last_insert_id,
                })
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(sql).fetch_all(pool).await?;
                let columns = if let Some(row) = rows.first() {
                    row.columns().iter().map(|c| c.name().to_string()).collect()
                } else {
                    Vec::new()
                };
                let data: Vec<Vec<serde_json::Value>> = rows
                    .iter()
                    .map(|row| {
                        columns
                            .iter()
                            .enumerate()
                            .map(|(i, _)| sqlite_value_to_json(row, i))
                            .collect()
                    })
                    .collect();
                // Extract last_insert_id for INSERT operations
                let last_insert_id = if sql.trim().to_uppercase().starts_with("INSERT") {
                    sqlx::query("SELECT last_insert_rowid() as id")
                        .fetch_optional(pool)
                        .await
                        .ok()
                        .flatten()
                        .and_then(|r| r.try_get::<i64, _>("id").ok().map(|v| v.to_string()))
                } else {
                    None
                };

                Ok(QueryResult {
                    columns,
                    rows: data,
                    row_count: rows.len(),
                    last_insert_id,
                })
            }
        }
    }

    pub async fn get_schema(
        &self,
        id: &str,
    ) -> Result<Vec<TableInfo>, sqlx::Error> {
        let pool = self.pools.get(id).ok_or_else(|| {
            sqlx::Error::Configuration("Database not connected".into())
        })?;

        match pool {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT table_name, column_name, data_type, is_nullable
                     FROM information_schema.columns
                     WHERE table_schema = 'public'
                     ORDER BY table_name, ordinal_position"
                )
                .fetch_all(pool)
                .await?;

                let mut tables: HashMap<String, Vec<ColumnInfo>> = HashMap::new();
                for row in rows {
                    let table: String = row.get("table_name");
                    let col = ColumnInfo {
                        name: row.get("column_name"),
                        data_type: row.get("data_type"),
                        nullable: row.get::<String, _>("is_nullable") == "YES",
                    };
                    tables.entry(table).or_default().push(col);
                }

                Ok(tables
                    .into_iter()
                    .map(|(name, columns)| TableInfo { name, columns })
                    .collect())
            }
            DatabasePool::MySql(pool) => {
                let rows = sqlx::query(
                    "SELECT table_name, column_name, data_type, is_nullable
                     FROM information_schema.columns
                     WHERE table_schema = DATABASE()
                     ORDER BY table_name, ordinal_position"
                )
                .fetch_all(pool)
                .await?;

                let mut tables: HashMap<String, Vec<ColumnInfo>> = HashMap::new();
                for row in rows {
                    let table: String = row.get("table_name");
                    let col = ColumnInfo {
                        name: row.get("column_name"),
                        data_type: row.get("data_type"),
                        nullable: row.get::<String, _>("is_nullable") == "YES",
                    };
                    tables.entry(table).or_default().push(col);
                }

                Ok(tables
                    .into_iter()
                    .map(|(name, columns)| TableInfo { name, columns })
                    .collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'"
                )
                .fetch_all(pool)
                .await?;

                let mut tables = Vec::new();
                for row in rows {
                    let name: String = row.get("name");
                    let cols = sqlx::query(&format!("PRAGMA table_info({})", name))
                        .fetch_all(pool)
                        .await?;

                    let columns: Vec<ColumnInfo> = cols
                        .iter()
                        .map(|c| ColumnInfo {
                            name: c.get("name"),
                            data_type: c.get("type"),
                            nullable: c.get::<i32, _>("notnull") == 0,
                        })
                        .collect();

                    tables.push(TableInfo { name, columns });
                }

                Ok(tables)
            }
        }
    }

        pub async fn get_table_columns(
        &self,
        id: &str,
        table_name: &str,
    ) -> Result<Vec<ColumnInfo>, sqlx::Error> {
        let pool = self.pools.get(id).ok_or_else(|| {
            sqlx::Error::Configuration("Database not connected".into())
        })?;

        match pool {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT column_name, data_type, is_nullable, column_default
                     FROM information_schema.columns
                     WHERE table_schema = 'public' AND table_name = $1
                     ORDER BY ordinal_position"
                )
                .bind(table_name)
                .fetch_all(pool)
                .await?;

                Ok(rows.iter().map(|row| ColumnInfo {
                    name: row.get("column_name"),
                    data_type: row.get("data_type"),
                    nullable: row.get::<String, _>("is_nullable") == "YES",
                }).collect())
            }
            DatabasePool::MySql(pool) => {
                let rows = sqlx::query(
                    "SELECT column_name, data_type, is_nullable, column_default
                     FROM information_schema.columns
                     WHERE table_schema = DATABASE() AND table_name = ?
                     ORDER BY ordinal_position"
                )
                .bind(table_name)
                .fetch_all(pool)
                .await?;

                Ok(rows.iter().map(|row| ColumnInfo {
                    name: row.get("column_name"),
                    data_type: row.get("data_type"),
                    nullable: row.get::<String, _>("is_nullable") == "YES",
                }).collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(&format!("PRAGMA table_info({})", table_name))
                    .fetch_all(pool)
                    .await?;

                Ok(rows.iter().map(|row| ColumnInfo {
                    name: row.get("name"),
                    data_type: row.get("type"),
                    nullable: row.get::<i32, _>("notnull") == 0,
                }).collect())
            }
        }
    }

    pub async fn get_table_indexes(
        &self,
        id: &str,
        table_name: &str,
    ) -> Result<Vec<IndexInfo>, sqlx::Error> {
        let pool = self.pools.get(id).ok_or_else(|| {
            sqlx::Error::Configuration("Database not connected".into())
        })?;

        match pool {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT indexname, indexdef 
                     FROM pg_indexes 
                     WHERE tablename = $1 AND schemaname = 'public'"
                )
                .bind(table_name)
                .fetch_all(pool)
                .await?;

                Ok(rows.iter().map(|row| {
                    let def: String = row.get("indexdef");
                    let is_unique = def.contains(" UNIQUE ");
                    let is_primary = def.contains(" PRIMARY KEY ");
                    let name: String = row.get("indexname");
                    IndexInfo {
                        name: name.clone(),
                        columns: Self::extract_columns_from_index_def(&def),
                        index_type: if def.contains("btree") { "btree".to_string() } else { "unknown".to_string() },
                        is_unique,
                        is_primary,
                    }
                }).collect())
            }
            DatabasePool::MySql(pool) => {
                let rows = sqlx::query(
                    "SELECT index_name, column_name, non_unique 
                     FROM information_schema.statistics 
                     WHERE table_schema = DATABASE() AND table_name = ?"
                )
                .bind(table_name)
                .fetch_all(pool)
                .await?;

                // Group by index_name
                let mut indexes: HashMap<String, (bool, Vec<String>)> = HashMap::new();
                for row in rows {
                    let name: String = row.get("index_name");
                    let col: String = row.get("column_name");
                    let non_unique: i64 = row.get("non_unique");
                    let entry = indexes.entry(name).or_insert((non_unique == 0, Vec::new()));
                    entry.1.push(col);
                }

                Ok(indexes.into_iter().map(|(name, (unique, cols))| IndexInfo {
                    name: name.clone(),
                    columns: cols,
                    index_type: "btree".to_string(),
                    is_unique: unique,
                    is_primary: name == "PRIMARY",
                }).collect())
            }
            DatabasePool::Sqlite(pool) => {
                let rows = sqlx::query(&format!("PRAGMA index_list({})", table_name))
                    .fetch_all(pool)
                    .await?;

                let mut indexes = Vec::new();
                for row in rows {
                    let name: String = row.get("name");
                    let unique: i32 = row.get("unique");
                    
                    let cols = sqlx::query(&format!("PRAGMA index_info({})", name))
                        .fetch_all(pool)
                        .await?;
                    
                    let columns: Vec<String> = cols.iter().map(|c| c.get::<String, _>("name")).collect();
                    
                    indexes.push(IndexInfo {
                        name: name.clone(),
                        columns,
                        index_type: "btree".to_string(),
                        is_unique: unique == 1,
                        is_primary: name.starts_with("sqlite_autoindex") && unique == 1,
                    });
                }
                Ok(indexes)
            }
        }
    }

    pub async fn get_table_constraints(
        &self,
        id: &str,
        table_name: &str,
    ) -> Result<Vec<ConstraintInfo>, sqlx::Error> {
        let pool = self.pools.get(id).ok_or_else(|| {
            sqlx::Error::Configuration("Database not connected".into())
        })?;

        match pool {
            DatabasePool::Postgres(pool) => {
                let rows = sqlx::query(
                    "SELECT conname, contype, pg_get_constraintdef(oid) as def
                     FROM pg_constraint
                     WHERE conrelid = $1::regclass"
                )
                .bind(table_name)
                .fetch_all(pool)
                .await?;

                Ok(rows.iter().map(|row| {
                    let con_type: String = row.get("contype");
                    ConstraintInfo {
                        name: row.get("conname"),
                        constraint_type: match con_type.as_str() {
                            "p" => "PRIMARY KEY".to_string(),
                            "f" => "FOREIGN KEY".to_string(),
                            "u" => "UNIQUE".to_string(),
                            "c" => "CHECK".to_string(),
                            _ => "UNKNOWN".to_string(),
                        },
                        columns: vec![], // Would need to parse def
                        definition: Some(row.get("def")),
                    }
                }).collect())
            }
            DatabasePool::MySql(pool) => {
                let rows = sqlx::query(
                    "SELECT constraint_name, constraint_type
                     FROM information_schema.table_constraints
                     WHERE table_schema = DATABASE() AND table_name = ?"
                )
                .bind(table_name)
                .fetch_all(pool)
                .await?;

                Ok(rows.iter().map(|row| ConstraintInfo {
                    name: row.get("constraint_name"),
                    constraint_type: row.get("constraint_type"),
                    columns: vec![], // Would need column mapping
                    definition: None,
                }).collect())
            }
            DatabasePool::Sqlite(pool) => {
                // SQLite constraints are in CREATE TABLE statement
                // Parse from table_info or use PRAGMA
                let rows = sqlx::query(
                    "SELECT sql FROM sqlite_master WHERE type='table' AND name = ?"
                )
                .bind(table_name)
                .fetch_all(pool)
                .await?;

                Ok(rows.iter().map(|row| ConstraintInfo {
                    name: format!("{}_table", table_name),
                    constraint_type: "TABLE".to_string(),
                    columns: vec![],
                    definition: row.try_get("sql").ok(),
                }).collect())
            }
        }
    }

    fn extract_columns_from_index_def(def: &str) -> Vec<String> {
        // Extract column names from "CREATE INDEX ... ON table (col1, col2)"
        if let Some(start) = def.find('(') {
            if let Some(end) = def.find(')') {
                let cols = &def[start+1..end];
                return cols.split(',').map(|s| s.trim().trim_matches('"').to_string()).collect();
            }
        }
        vec![]
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
    pub last_insert_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TableInfo {
    pub name: String,
    pub columns: Vec<ColumnInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexInfo {
    pub name: String,
    pub columns: Vec<String>,
    pub index_type: String,
    pub is_unique: bool,
    pub is_primary: bool,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConstraintInfo {
    pub name: String,
    pub constraint_type: String,
    pub columns: Vec<String>,
    pub definition: Option<String>,
}

fn pg_value_to_json(row: &sqlx::postgres::PgRow, i: usize) -> serde_json::Value {
    if let Ok(v) = row.try_get::<String, _>(i) {
        serde_json::Value::String(v)
    } else if let Ok(v) = row.try_get::<i64, _>(i) {
        serde_json::Value::Number(v.into())
    } else if let Ok(v) = row.try_get::<f64, _>(i) {
        serde_json::json!(v)
    } else if let Ok(v) = row.try_get::<bool, _>(i) {
        serde_json::Value::Bool(v)
    } else if let Ok(v) = row.try_get::<chrono::DateTime<chrono::Utc>, _>(i) {
        serde_json::Value::String(v.to_rfc3339())
    } else if let Ok(v) = row.try_get::<chrono::NaiveDateTime, _>(i) {
        serde_json::Value::String(v.to_string())
    } else if let Ok(v) = row.try_get::<serde_json::Value, _>(i) {
        v
    } else {
        serde_json::Value::Null
    }
}

fn mysql_value_to_json(row: &sqlx::mysql::MySqlRow, i: usize) -> serde_json::Value {
    if let Ok(v) = row.try_get::<String, _>(i) {
        serde_json::Value::String(v)
    } else if let Ok(v) = row.try_get::<i64, _>(i) {
        serde_json::Value::Number(v.into())
    } else if let Ok(v) = row.try_get::<f64, _>(i) {
        serde_json::json!(v)
    } else if let Ok(v) = row.try_get::<bool, _>(i) {
        serde_json::Value::Bool(v)
    } else if let Ok(v) = row.try_get::<chrono::DateTime<chrono::Utc>, _>(i) {
        serde_json::Value::String(v.to_rfc3339())
    } else if let Ok(v) = row.try_get::<chrono::NaiveDateTime, _>(i) {
        serde_json::Value::String(v.to_string())
    } else {
        serde_json::Value::Null
    }
}

fn sqlite_value_to_json(row: &sqlx::sqlite::SqliteRow, i: usize) -> serde_json::Value {
    if let Ok(v) = row.try_get::<String, _>(i) {
        serde_json::Value::String(v)
    } else if let Ok(v) = row.try_get::<i64, _>(i) {
        serde_json::Value::Number(v.into())
    } else if let Ok(v) = row.try_get::<f64, _>(i) {
        serde_json::json!(v)
    } else if let Ok(v) = row.try_get::<bool, _>(i) {
        serde_json::Value::Bool(v)
    } else {
        serde_json::Value::Null
    }
}
