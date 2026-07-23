//! bennett keys — manage durable API keys for external app access
//! Talks to the engine's /api/keys REST endpoints (engine/src/api/api_keys.rs)

use clap::Subcommand;
use serde::Deserialize;
use serde_json::json;

#[derive(Debug, Subcommand)]
pub enum KeysCommand {
    /// Create a new durable API key for a database
    Create {
        #[arg(long)]
        database_id: String,
        /// Human-readable name (e.g. "oshocks-backend")
        #[arg(long)]
        name: String,
        /// Permission level: ro, rw, or adm
        #[arg(long, default_value = "ro")]
        permission: String,
        /// Comma-separated table allowlist ("*" for all)
        #[arg(long, default_value = "*")]
        tables: String,
        /// Max rows returned per query (default: 1000)
        #[arg(long)]
        max_rows: Option<i32>,
        /// Query timeout in seconds (default: 30)
        #[arg(long)]
        timeout_secs: Option<i32>,
    },
    /// List API keys, optionally filtered by database
    List {
        #[arg(long)]
        database_id: Option<String>,
    },
    /// Revoke an API key permanently
    Revoke {
        /// API key ID (from `bennett keys list`)
        id: String,
    },
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    success: bool,
    data: Option<T>,
    error: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateApiKeyResponse {
    id: String,
    key: String,
    name: String,
    permission: String,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct ApiKeyInfo {
    id: String,
    name: String,
    permission: String,
    last_used_at: Option<String>,
    revoked: bool,
    key_preview: String,
}

#[derive(Debug, Deserialize)]
struct ListApiKeysResponse {
    keys: Vec<ApiKeyInfo>,
    total: usize,
}

pub async fn handle(cmd: KeysCommand, engine_url: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();

    match cmd {
        KeysCommand::Create { database_id, name, permission, tables, max_rows, timeout_secs } => {
            let tables_vec: Vec<String> = tables.split(',').map(|s| s.trim().to_string()).collect();
            let body = json!({
                "database_id": database_id,
                "name": name,
                "permission": permission,
                "tables": tables_vec,
                "max_rows": max_rows,
                "timeout_secs": timeout_secs,
            });

            let resp = client.post(format!("{}/api/keys", engine_url)).json(&body).send().await?;
            let parsed: ApiResponse<CreateApiKeyResponse> = resp.json().await?;

            match parsed.data {
                Some(k) => {
                    println!("API key created successfully.\n");
                    println!("  ID:         {}", k.id);
                    println!("  Name:       {}", k.name);
                    println!("  Permission: {}", k.permission);
                    println!("  Created:    {}", k.created_at);
                    println!();
                    println!("  Key: {}", k.key);
                    println!();
                    println!("Save this key now — it will not be shown again.");
                }
                None => anyhow::bail!("Failed to create key: {}", parsed.error.unwrap_or_else(|| "unknown error".to_string())),
            }
        }

        KeysCommand::List { database_id } => {
            let mut req = client.get(format!("{}/api/keys", engine_url));
            if let Some(db) = &database_id {
                req = req.query(&[("database_id", db)]);
            }
            let parsed: ApiResponse<ListApiKeysResponse> = req.send().await?.json().await?;

            match parsed.data {
                Some(list) if list.total > 0 => {
                    println!("{:<36}  {:<20}  {:<10}  {:<15}  {:<8}  {}", "ID", "NAME", "PERMISSION", "KEY", "REVOKED", "LAST USED");
                    for k in list.keys {
                        println!(
                            "{:<36}  {:<20}  {:<10}  {:<15}  {:<8}  {}",
                            k.id, k.name, k.permission, k.key_preview,
                            if k.revoked { "yes" } else { "no" },
                            k.last_used_at.unwrap_or_else(|| "never".to_string()),
                        );
                    }
                }
                _ => println!("No API keys found."),
            }
        }

        KeysCommand::Revoke { id } => {
            let resp = client.delete(format!("{}/api/keys/{}", engine_url, id)).send().await?;
            let parsed: ApiResponse<serde_json::Value> = resp.json().await?;
            if parsed.success {
                println!("Revoked API key {}", id);
            } else {
                anyhow::bail!("Failed to revoke key: {}", parsed.error.unwrap_or_else(|| "unknown error".to_string()));
            }
        }
    }

    Ok(())
}
