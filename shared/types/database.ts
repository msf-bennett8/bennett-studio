// ============================================================================
// Database Source
// ============================================================================

export type DatabaseSource = 'bennett' | 'local';

// ============================================================================
// Database Instance
// ============================================================================

export interface DatabaseCredentials {
  username: string;
  password: string;
  database: string;
}

export interface DatabaseInstance {
  id: string;
  name: string;
  type: 'postgres' | 'mysql' | 'mariadb' | 'sqlite' | 'redis';
  version: string;
  status: 'running' | 'stopped' | 'starting' | 'error';
  port: number;
  size: string;
  created_at: string;
  container_id?: string;
  volume_name?: string;
  env_vars?: [string, string][];
  source?: DatabaseSource;
  is_discovered?: boolean;
  credentials?: DatabaseCredentials;
  is_unlocked?: boolean;
}

export interface UnlockDatabaseRequest {
  username: string;
  password: string;
  database: string;
}

export interface DatabaseStatusResponse {
  id: string;
  is_connected: boolean;
  is_unlocked: boolean;
  has_credentials: boolean;
  last_error?: string;
}

export interface EnvFileSuggestion {
  source: string;
  username?: string;
  password?: string;
  database?: string;
  host?: string;
  port?: string;
}

export interface CreateDatabaseRequest {
  name: string;
  type: string;
  version: string;
}

export interface UpdateDatabaseRequest {
  name?: string;
  status?: 'running' | 'stopped' | 'starting' | 'error';
}

// ============================================================================
// Schema Types
// ============================================================================

export interface ColumnSchema {
  name: string;
  data_type: string;
  nullable: boolean;
}

export interface TableSchema {
  name: string;
  columns: ColumnSchema[];
}

// ============================================================================
// Query Types
// ============================================================================

export interface ExecuteQueryRequest {
  sql: string;
}

export interface QueryResult {
  columns: string[];
  rows: any[][];
  row_count: number;
  execution_time_ms?: number;
}

// ============================================================================
// Data Grid Types
// ============================================================================

export interface TableDataRequest {
  table: string;
  limit?: number;
  offset?: number;
  order_by?: string;
  order_dir?: 'ASC' | 'DESC';
  filter?: string;
}

export interface TableDataResponse {
  columns: string[];
  rows: any[][];
  row_count: number;
  total_count: number;
}

export interface UpdateRowRequest {
  table: string;
  primary_key: any;
  primary_key_column: string;
  data: Record<string, any>;
}

export interface DeleteRowRequest {
  table: string;
  primary_key: any;
  primary_key_column: string;
}
