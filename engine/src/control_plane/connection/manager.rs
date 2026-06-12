use std::collections::HashMap;
use tracing::info;
use sqlx::{Column, Row};

use crate::models::database::DatabaseInstance;

pub struct ConnectionManager {
    pools: HashMap<String, DatabasePool>,
}

pub enum DatabasePool {
    Postgres(sqlx::PgPool),
    MySql(sqlx::MySqlPool),
    Sqlite(sqlx::SqlitePool),
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            pools: HashMap::new(),
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
                let url = format!(
                    "mysql://bennett:bennett_secret@localhost:{}/bennett",
                    instance.port
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

    pub async fn execute(
        &self,
        id: &str,
        sql: &str,
    ) -> Result<QueryResult, sqlx::Error> {
        let pool = self.pools.get(id).ok_or_else(|| {
            sqlx::Error::Configuration("Database not connected".into())
        })?;

        match pool {
            DatabasePool::Postgres(pool) => {
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
                            .map(|(i, _)| pg_value_to_json(row, i))
                            .collect()
                    })
                    .collect();
                Ok(QueryResult {
                    columns,
                    rows: data,
                    row_count: rows.len(),
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
                Ok(QueryResult {
                    columns,
                    rows: data,
                    row_count: rows.len(),
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
                Ok(QueryResult {
                    columns,
                    rows: data,
                    row_count: rows.len(),
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
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<serde_json::Value>>,
    pub row_count: usize,
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
