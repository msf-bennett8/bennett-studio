import type {
  RemoteConnection,
  RemoteSchemaCache,
  RemoteQueryResult,
  RemoteQueryHistory,
  ValidateShareResponse,
  SharePermission,
  TableSchema,
  AutocompleteSuggestion,
} from '@bennett/shared';

// Import SDK from shared package
import { BennettShareClient, createClient } from '@bennett/sdk';

const SCHEMA_TTL_MS = 30000; // 30 seconds cache TTL

class RemoteApiService {
  private clients: Map<string, BennettShareClient> = new Map();
  private schemaCache: Map<string, RemoteSchemaCache> = new Map();
  private queryHistory: Map<string, RemoteQueryHistory[]> = new Map();

  /**
   * Parse a share URL and create connection
   */
  parseShareUrl(url: string): { code: string; token: string; baseUrl: string } | null {
    try {
      const urlObj = new URL(url);
      
      // Extract code from path: /db/ACQPFDAQ7P
      const pathMatch = urlObj.pathname.match(/\/db\/([A-Z0-9]+)/i);
      if (!pathMatch) return null;
      
      const code = pathMatch[1].toUpperCase();
      
      // Extract token from query: ?t=eyJhbG...
      const token = urlObj.searchParams.get('t');
      if (!token) return null;
      
      // Base URL is everything before /db/
      const baseUrl = url.substring(0, url.indexOf('/db/'));
      
      return { code, token, baseUrl };
    } catch {
      return null;
    }
  }

  /**
   * Create or get existing client for a share
   */
  private getClient(connection: RemoteConnection): BennettShareClient {
    const cacheKey = connection.code;
    
    if (!this.clients.has(cacheKey)) {
      const client = createClient(connection.code, connection.token, connection.baseUrl);
      this.clients.set(cacheKey, client);
    }
    
    return this.clients.get(cacheKey)!;
  }

  /**
   * Validate a share before connecting
   */
  async validateShare(url: string): Promise<ValidateShareResponse> {
    const parsed = this.parseShareUrl(url);
    if (!parsed) {
      throw new Error('Invalid share URL format. Expected: https://host/db/CODE?t=TOKEN');
    }
    
    const client = createClient(parsed.code, parsed.token, parsed.baseUrl);
    const response = await client.getSchema();
    
    // If getSchema succeeds, share is valid
    return {
      valid: true,
      code: parsed.code,
      db_id: response.databaseName || parsed.code,
      permission: 'ro' as SharePermission, // Will be updated from actual validation
      tables: response.tables.map(t => t.name),
      expires_at: new Date(Date.now() + 24 * 60 * 60 * 1000).toISOString(),
      host_online: true,
    };
  }

  /**
   * Connect to a remote share and return connection info
   */
  async connect(url: string): Promise<RemoteConnection> {
    const parsed = this.parseShareUrl(url);
    if (!parsed) {
      throw new Error('Invalid share URL');
    }
    
    const connection: RemoteConnection = {
      id: `conn-${Date.now()}`,
      code: parsed.code,
      token: parsed.token,
      baseUrl: parsed.baseUrl,
      dbId: '',
      dbName: '',
      dbType: '',
      permission: 'ro',
      tables: [],
      connectedAt: new Date().toISOString(),
      lastActivity: new Date().toISOString(),
      status: 'connecting',
    };
    
    try {
      const client = this.getClient(connection);
      const schema = await client.getSchema();
      
      if (!schema.success) {
        throw new Error(schema.error || 'Failed to fetch schema');
      }
      
      connection.dbId = schema.databaseName || parsed.code;
      connection.dbName = schema.databaseName || 'Remote Database';
      connection.dbType = schema.databaseType || 'unknown';
      connection.tables = schema.tables.map(t => t.name);
      connection.status = 'connected';
      connection.lastActivity = new Date().toISOString();
      
      // Cache schema
      this.cacheSchema(connection.code, schema.tables);
      
      return connection;
    } catch (error) {
      connection.status = 'error';
      connection.error = error instanceof Error ? error.message : 'Connection failed';
      throw error;
    }
  }

  /**
   * Disconnect and cleanup
   */
  disconnect(connectionId: string): void {
    // Find connection by ID and remove client
    for (const [code, client] of this.clients) {
      // Note: In real implementation, track connection ID to client mapping
      this.clients.delete(code);
    }
    this.schemaCache.delete(connectionId);
    this.queryHistory.delete(connectionId);
  }

  /**
   * Fetch schema with caching
   */
  async fetchSchema(connection: RemoteConnection, forceRefresh = false): Promise<TableSchema[]> {
    const cached = this.schemaCache.get(connection.code);
    
    if (!forceRefresh && cached) {
      const expiresAt = new Date(cached.expiresAt).getTime();
      if (Date.now() < expiresAt) {
        return cached.schema;
      }
    }
    
    const client = this.getClient(connection);
    const response = await client.getSchema();
    
    if (!response.success) {
      throw new Error(response.error || 'Failed to fetch schema');
    }
    
    this.cacheSchema(connection.code, response.tables);
    return response.tables;
  }

  private cacheSchema(code: string, schema: TableSchema[]): void {
    const now = Date.now();
    this.schemaCache.set(code, {
      code,
      schema,
      fetchedAt: new Date(now).toISOString(),
      expiresAt: new Date(now + SCHEMA_TTL_MS).toISOString(),
      ttlSeconds: SCHEMA_TTL_MS / 1000,
    });
  }

  /**
   * Execute query on remote database
   */
  async executeQuery(connection: RemoteConnection, sql: string): Promise<RemoteQueryResult> {
    const start = performance.now();
    
    const client = this.getClient(connection);
    const response = await client.query(sql);
    
    const executionTimeMs = Math.round(performance.now() - start);
    
    // Record in history
    const history: RemoteQueryHistory = {
      id: `query-${Date.now()}`,
      sql,
      executedAt: new Date().toISOString(),
      executionTimeMs: response.executionTimeMs || executionTimeMs,
      rowCount: response.rowCount,
      status: response.success ? 'success' : 'error',
      error: response.error,
    };
    
    const existing = this.queryHistory.get(connection.code) || [];
    this.queryHistory.set(connection.code, [history, ...existing].slice(0, 100));
    
    connection.lastActivity = new Date().toISOString();
    
    return {
      columns: response.columns,
      rows: response.rows,
      rowCount: response.rowCount,
      executionTimeMs: response.executionTimeMs || executionTimeMs,
      error: response.error,
    };
  }

  /**
   * Execute write query (INSERT/UPDATE/DELETE)
   */
  async executeWrite(connection: RemoteConnection, sql: string): Promise<{ rowsAffected: number; error?: string }> {
    if (connection.permission === 'ro') {
      throw new Error('Write operations not allowed on read-only share');
    }
    
    const client = this.getClient(connection);
    const response = await client.write(sql);
    
    connection.lastActivity = new Date().toISOString();
    
    return {
      rowsAffected: response.rowsAffected,
      error: response.error,
    };
  }

  /**
   * Get autocomplete suggestions from cached schema
   */
  getAutocompleteSuggestions(connection: RemoteConnection, prefix: string): AutocompleteSuggestion[] {
    const cached = this.schemaCache.get(connection.code);
    if (!cached) return [];
    
    const suggestions: AutocompleteSuggestion[] = [];
    const lowerPrefix = prefix.toLowerCase();
    
    // Table suggestions
    for (const table of cached.schema) {
      if (table.name.toLowerCase().startsWith(lowerPrefix)) {
        suggestions.push({
          type: 'table',
          label: table.name,
          detail: `${table.columns.length} columns`,
          insertText: table.name,
          sortText: `1_${table.name}`,
        });
      }
      
      // Column suggestions (only if prefix matches column or table is context)
      for (const col of table.columns) {
        if (col.name.toLowerCase().startsWith(lowerPrefix)) {
          suggestions.push({
            type: 'column',
            label: col.name,
            detail: `${table.name}.${col.name} — ${col.dataType}`,
            insertText: col.name,
            sortText: `2_${col.name}`,
            documentation: `Nullable: ${col.nullable}, PK: ${col.isPrimaryKey}`,
          });
        }
      }
    }
    
    // SQL keywords
    const keywords = ['SELECT', 'FROM', 'WHERE', 'JOIN', 'INNER', 'LEFT', 'RIGHT', 'ON', 'GROUP BY', 'ORDER BY', 'HAVING', 'LIMIT', 'OFFSET', 'INSERT INTO', 'UPDATE', 'DELETE FROM', 'CREATE TABLE', 'ALTER TABLE', 'DROP TABLE', 'AND', 'OR', 'NOT', 'IN', 'EXISTS', 'BETWEEN', 'LIKE', 'IS NULL', 'IS NOT NULL', 'COUNT', 'SUM', 'AVG', 'MIN', 'MAX', 'DISTINCT', 'AS', 'UNION', 'ALL'];
    
    for (const kw of keywords) {
      if (kw.toLowerCase().startsWith(lowerPrefix)) {
        suggestions.push({
          type: 'keyword',
          label: kw,
          insertText: kw,
          sortText: `3_${kw}`,
        });
      }
    }
    
    // Sort and limit
    suggestions.sort((a, b) => (a.sortText || a.label).localeCompare(b.sortText || b.label));
    return suggestions.slice(0, 50);
  }

  /**
   * Get query history for a connection
   */
  getQueryHistory(connection: RemoteConnection): RemoteQueryHistory[] {
    return this.queryHistory.get(connection.code) || [];
  }

  /**
   * Export query results
   */
  async exportQuery(connection: RemoteConnection, sql: string, format: 'csv' | 'json'): Promise<string> {
    const client = this.getClient(connection);
    
    if (format === 'csv') {
      const response = await client.exportCsv(sql);
      if (!response.success) throw new Error(response.error || 'Export failed');
      return atob(response.data); // Base64 decode
    } else {
      const response = await client.exportJson(sql);
      if (!response.success) throw new Error(response.error || 'Export failed');
      return atob(response.data);
    }
  }
}

// Singleton instance
export const remoteApi = new RemoteApiService();
