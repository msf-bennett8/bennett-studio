// ============================================================================
// Sharing Types — Phase 1: DBaaS Share System
// ============================================================================

export type SharePermission = 'ro' | 'rw' | 'adm';

export interface ShareLink {
  code: string;
  url: string;
  db_id: string;
  db_name: string;
  db_type: string;
  permission: SharePermission;
  tables: string[];
  expires_at: string;
  created_at: string;
  guest_count: number;
  pinned: boolean;
  status: 'active' | 'expired' | 'revoked';
}

export interface ShareSession {
  id: string;
  database_id: string;
  guest_count: number;
  active: boolean;
  created_at: string;
}

// Phase 1: API request/response types
export interface CreateShareRequest {
  database_id: string;
  permission?: SharePermission;
  tables?: string[];
  cols?: Record<string, string[]>;
  rls?: string;
  duration_hours?: number;
}

export interface CreateShareResponse {
  code: string;
  url: string;
  token: string;
  expires_at: string;
}

export interface ValidateShareRequest {
  code: string;
  token: string;
}

export interface ValidateShareResponse {
  valid: boolean;
  code: string;
  db_id: string;
  permission: SharePermission;
  tables: string[];
  expires_at: string;
  host_online: boolean;
}

export interface RevokeShareRequest {
  code: string;
  reason?: string;
}

export interface DeleteShareRequest {
  code: string;
}

export interface ListSharesResponse {
  shares: ShareLink[];
  total: number;
}

// ============================================================================
// API Key Types — durable credentials for external app access (/api/v1)
// Distinct from ShareLink: no expiry until explicitly revoked.
// ============================================================================

export interface CreateApiKeyRequest {
  database_id: string;
  name: string;
  permission?: SharePermission;
  tables?: string[];
  cols?: Record<string, string[]>;
  rls?: string;
  max_rows?: number;
  timeout_secs?: number;
  /** Enable a MySQL/Postgres wire-protocol credential pair for this key */
  enable_wire_access?: boolean;
  /** Custom wire username — auto-generated as "bennett_<name>" if omitted */
  wire_username?: string;
  /** Custom wire password — auto-generated if omitted */
  wire_password?: string;
}

export interface CreateApiKeyResponse {
  id: string;
  key: string; // plaintext — shown once
  name: string;
  permission: SharePermission;
  created_at: string;
  /** Present only if wire access was enabled — shown once, never retrievable again */
  wire_username?: string;
  wire_password?: string;
}

export interface ApiKeyInfo {
  id: string;
  name: string;
  db_id: string;
  permission: SharePermission;
  tables: string[];
  created_at: string;
  last_used_at: string | null;
  revoked: boolean;
  key_preview: string;
  max_rows: number;
  timeout_secs: number;
  wire_enabled: boolean;
  wire_username: string | null;
}

export interface ListApiKeysResponse {
  keys: ApiKeyInfo[];
  total: number;
}

// ============================================================================
// Phase 3: Guest/Remote Connection Types
// ============================================================================

export interface RemoteConnection {
  id: string;
  code: string;
  token: string;
  baseUrl: string;
  dbId: string;
  dbName: string;
  dbType: string;
  permission: SharePermission;
  tables: string[];
  connectedAt: string;
  lastActivity: string;
  status: 'connecting' | 'connected' | 'error' | 'disconnected';
  error?: string;
  /** Original share URL for reconnection on refresh */
  shareUrl: string;
}

export interface RemoteSchemaCache {
  code: string;
  schema: TableSchema[];
  fetchedAt: string;
  expiresAt: string;
  ttlSeconds: number;
}

export interface TableSchema {
  name: string;
  columns: ColumnSchema[];
  indexes: IndexSchema[];
  constraints: ConstraintSchema[];
  estimatedRowCount: number;
  tableSize?: string;
}

export interface ColumnSchema {
  name: string;
  dataType: string;
  nullable: boolean;
  defaultValue?: string;
  isPrimaryKey: boolean;
  isForeignKey: boolean;
  foreignKeyReference?: string;
  comment?: string;
}

export interface IndexSchema {
  name: string;
  columns: string[];
  indexType: string;
  isUnique: boolean;
  isPrimary: boolean;
}

export interface ConstraintSchema {
  name: string;
  constraintType: string;
  columns: string[];
  definition?: string;
}

export interface RemoteQueryResult {
  columns: string[];
  rows: any[][];
  rowCount: number;
  executionTimeMs: number;
  error?: string;
}

export interface RemoteQueryHistory {
  id: string;
  sql: string;
  executedAt: string;
  executionTimeMs: number;
  rowCount: number;
  status: 'success' | 'error';
  error?: string;
}

export interface AutocompleteSuggestion {
  type: 'table' | 'column' | 'keyword' | 'function';
  label: string;
  detail?: string;
  insertText: string;
  sortText?: string;
  documentation?: string;
}

// ============================================================================
// Token Vault Types — Secure share token storage
// ============================================================================

export interface StoredToken {
  code: string;
  token: string;
  dbId: string;
  dbName: string;
  createdAt: string;
  expiresAt: string;
}

export interface TokenVault {
  getToken(code: string): Promise<string | null>;
  setToken(token: StoredToken): Promise<void>;
  removeToken(code: string): Promise<void>;
  listTokens(): Promise<StoredToken[]>;
  clear(): Promise<void>;
  status?(): Promise<VaultStatus>;
}

export interface VaultStatus {
  available: boolean;
  type: 'tauri_secure' | 'indexeddb_encrypted' | 'memory';
  initialized: boolean;
}
