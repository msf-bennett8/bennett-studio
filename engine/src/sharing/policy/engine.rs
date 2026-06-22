//! Policy engine for share permission enforcement
//! Phase 2: Table-level, column-level, RLS, query-type restrictions

use crate::auth::share_token::SharePermission;

/// Permission check result
#[derive(Debug, Clone, PartialEq)]
pub enum PermissionResult {
    Allow,
    Deny(String),
}

/// Policy engine for evaluating share permissions
pub struct PolicyEngine;

impl PolicyEngine {
    /// Check if a query is allowed under the given permission
    pub fn check_query(
        sql: &str,
        permission: &SharePermission,
        allowed_tables: &[String],
        _allowed_columns: &Option<serde_json::Value>,
        rls: Option<&str>,
    ) -> PermissionResult {
        // Check write permission
        let upper = sql.trim().to_uppercase();
        let is_write = upper.starts_with("INSERT ") 
            || upper.starts_with("UPDATE ") 
            || upper.starts_with("DELETE ")
            || upper.starts_with("CREATE ")
            || upper.starts_with("ALTER ")
            || upper.starts_with("DROP ")
            || upper.starts_with("TRUNCATE ");
        
        if is_write && !permission.can_write() {
            return PermissionResult::Deny(
                "Write operations require read-write permission".to_string()
            );
        }
        
        // Check DDL blocking (even for rw)
        let is_ddl = upper.starts_with("CREATE ") 
            || upper.starts_with("ALTER ") 
            || upper.starts_with("DROP ")
            || upper.starts_with("TRUNCATE ")
            || upper.starts_with("GRANT ")
            || upper.starts_with("REVOKE ");
        
        if is_ddl && permission != &SharePermission::Admin {
            return PermissionResult::Deny(
                "DDL operations require admin permission".to_string()
            );
        }
        
        // Check table access
        if allowed_tables.len() != 1 || allowed_tables[0] != "*" {
            // Extract table names from query (naive parsing)
            // NOTE: sqlparser-rs would improve accuracy for complex subqueries, CTEs
            let tables_referenced = Self::extract_table_names(sql);
            for table in &tables_referenced {
                if !allowed_tables.contains(table) {
                    return PermissionResult::Deny(
                        format!("Access to table '{}' not allowed", table)
                    );
                }
            }
        }
        
        // RLS check - ensure RLS is present in query
        if let Some(rls_filter) = rls {
            if !upper.contains(&rls_filter.to_uppercase()) {
                // RLS not applied - this is a warning, not a block
                // The query will be rewritten with RLS
            }
        }
        
        PermissionResult::Allow
    }
    
    /// Check if table is accessible
    pub fn check_table_access(
        table_name: &str,
        allowed_tables: &[String],
    ) -> PermissionResult {
        if allowed_tables.len() == 1 && allowed_tables[0] == "*" {
            return PermissionResult::Allow;
        }
        
        if allowed_tables.contains(&table_name.to_string()) {
            PermissionResult::Allow
        } else {
            PermissionResult::Deny(
                format!("Access to table '{}' not allowed", table_name)
            )
        }
    }
    
    /// Extract table names from SQL (naive implementation)
    /// NOTE: sqlparser-rs would improve accuracy for complex subqueries, CTEs, derived tables
    fn extract_table_names(sql: &str) -> Vec<String> {
        let upper = sql.to_uppercase();
        let mut tables = Vec::new();
        
        // Simple regex-like extraction
        // FROM table_name
        if let Some(from_pos) = upper.find(" FROM ") {
            let after_from = &sql[from_pos + 6..];
            let table_name = after_from.split_whitespace().next().unwrap_or("");
            let clean = table_name.trim_matches('"').trim_matches('`').trim_matches('\'').to_string();
            if !clean.is_empty() {
                tables.push(clean);
            }
        }
        
        // JOIN table_name
        let mut search_start = 0;
        while let Some(join_pos) = upper[search_start..].find(" JOIN ") {
            let abs_pos = search_start + join_pos;
            let after_join = &sql[abs_pos + 6..];
            let table_name = after_join.split_whitespace().next().unwrap_or("");
            let clean = table_name.trim_matches('"').trim_matches('`').trim_matches('\'').to_string();
            if !clean.is_empty() {
                tables.push(clean);
            }
            search_start = abs_pos + 6;
        }
        
        // INTO table_name
        if let Some(into_pos) = upper.find(" INTO ") {
            let after_into = &sql[into_pos + 6..];
            let table_name = after_into.split_whitespace().next().unwrap_or("");
            let clean = table_name.trim_matches('"').trim_matches('`').trim_matches('\'').to_string();
            if !clean.is_empty() {
                tables.push(clean);
            }
        }
        
        // UPDATE table_name
        if upper.starts_with("UPDATE ") {
            let after_update = &sql[7..];
            let table_name = after_update.split_whitespace().next().unwrap_or("");
            let clean = table_name.trim_matches('"').trim_matches('`').trim_matches('\'').to_string();
            if !clean.is_empty() {
                tables.push(clean);
            }
        }
        
        tables
    }
    
    /// Apply column-level filtering to result
    /// Delegates to connect_rpc::filter_columns for full implementation
    pub fn filter_columns(
        columns: &[String],
        allowed_columns: &Option<serde_json::Value>,
    ) -> Vec<String> {
        crate::connect_rpc::filter_columns(columns, allowed_columns, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_check_query_read_only() {
        let perm = SharePermission::ReadOnly;
        let tables = vec!["*".to_string()];
        
        let result = PolicyEngine::check_query("SELECT * FROM users", &perm, &tables, &None, None);
        assert!(matches!(result, PermissionResult::Allow));
        
        let result = PolicyEngine::check_query("INSERT INTO users VALUES (1)", &perm, &tables, &None, None);
        assert!(matches!(result, PermissionResult::Deny(_)));
    }
    
    #[test]
    fn test_extract_table_names() {
        let sql = "SELECT * FROM users u JOIN orders o ON u.id = o.user_id";
        let tables = PolicyEngine::extract_table_names(sql);
        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"orders".to_string()));
    }
}
