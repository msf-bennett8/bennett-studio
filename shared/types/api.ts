// ============================================================================
// Database Types
// ============================================================================

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
// Table / Schema Types
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
// Data Grid Types (NEW — Railway-style editor)
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
// API Response Wrapper
// ============================================================================

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

// ============================================================================
// Sharing Types
// ============================================================================

export interface ShareLink {
  id: string;
  database_id: string;
  token: string;
  expires_at?: string;
  permissions: 'read' | 'write' | 'admin';
}

export interface ShareSession {
  id: string;
  database_id: string;
  guest_count: number;
  active: boolean;
  created_at: string;
}
