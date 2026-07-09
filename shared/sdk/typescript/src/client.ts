/**
 * Bennett Studio Connect-RPC Client SDK
 * TypeScript client for querying shared databases
 * 
 * Usage:
 *   const client = new BennettShareClient({ code: 'ACQPFDAQ7P', token: 'eyJ...' });
 *   const result = await client.query('SELECT * FROM users LIMIT 10');
 */

import { resolveHost, preloadHosts, resolveRelayUrl } from './resolver';
import { P2PConnection } from './p2p';

export interface BennettClientConfig {
  /** Share code (e.g., 'ACQPFDAQ7P') */
  code: string;
  /** JWT token from share URL */
  token: string;
  /** Base URL of the host engine (default: auto-detect from code) */
  baseUrl?: string;
  /** Request timeout in ms (default: 30000) */
  timeout?: number;
}

export interface QueryResult {
  success: boolean;
  columns: string[];
  rows: any[][];
  rowCount: number;
  executionTimeMs: number;
  error?: string;
}

export interface WriteResult {
  success: boolean;
  rowsAffected: number;
  lastInsertId?: string;
  executionTimeMs: number;
  error?: string;
}

export interface SchemaResult {
  success: boolean;
  tables: TableSchema[];
  databaseName: string;
  databaseType: string;
  databaseVersion: string;
  error?: string;
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

export interface ExportResult {
  success: boolean;
  data: string; // Base64 encoded
  isLast: boolean;
  totalRows: number;
  chunkIndex: number;
  error?: string;
}

export class BennettShareClient {
  private code: string;
  private token: string;
  private baseUrl: string;
  private timeout: number;
  private resolved: boolean;

  /** gRPC-Web client for HTTP/2 streaming */
  private grpcClient?: any;

  /** P2P WebRTC connection (null if using relay/direct) */
  private p2pConnection: P2PConnection | null = null;

  /** Connection mode: 'p2p' | 'relay' | 'direct' */
  private connectionMode: 'p2p' | 'relay' | 'direct' = 'relay';

  /** Whether P2P was attempted and failed */
  private p2pFailed: boolean = false;

  /** Auto-retry configuration */
  private maxRetries: number = 3;
  private retryDelayMs: number = 1000;
  private connectionState: 'connecting' | 'connected' | 'disconnected' | 'error' = 'disconnected';
  private stateListeners: Set<(state: string) => void> = new Set();

  constructor(config: BennettClientConfig) {
    this.code = config.code;
    this.token = config.token;
    this.baseUrl = config.baseUrl || 'https://placeholder';
    this.timeout = config.timeout || 30000;
    this.resolved = !!config.baseUrl;

    // Detect connection mode from JWT
    const claims = decodeJwtPayload(config.token);
    if (claims?.ice) {
      this.connectionMode = 'p2p';
      console.log(`[BennettSDK] P2P mode detected — ICE candidates present`);
    } else if (config.baseUrl && config.baseUrl !== 'https://placeholder') {
      this.connectionMode = 'direct';
    }

    // Initialize gRPC-Web client if host supports it
    if (config.baseUrl && config.baseUrl !== 'https://placeholder') {
      try {
        const { BennettGrpcWebClient } = require('./grpcClient');
        this.grpcClient = new BennettGrpcWebClient({
          host: this.baseUrl.replace(/^https?:\/\//, ''),
          tls: this.baseUrl.startsWith('https'),
        });
      } catch {
        this.grpcClient = null;
      }
    }
  }
  
  /** Ensure connection is established before making requests */
  private async ensureResolved(): Promise<void> {
    if (this.resolved && this.p2pConnection?.isConnected()) return;

    // PHASE 2: Try P2P first if ICE candidates available
    if (this.connectionMode === 'p2p' && !this.p2pFailed) {
      try {
        const claims = decodeJwtPayload(this.token);
        if (claims?.ice) {
          console.log(`[BennettSDK] Attempting P2P connection...`);
          this.p2pConnection = new P2PConnection();
          await this.p2pConnection.connect(claims.ice, claims.sub, getFirebaseUrl());
          console.log(`[BennettSDK] P2P connected successfully`);
          this.resolved = true;
          return;
        }
      } catch (e) {
        // PHASE G: Handle relay fallback gracefully
        if (e instanceof Error && e.name === 'P2PRelayFallbackError') {
          console.log(`[BennettSDK] P2P signaled relay fallback — switching to relay mode`);
        } else {
          console.warn(`[BennettSDK] P2P failed, will fallback to relay:`, e);
        }
        this.p2pFailed = true;
        this.p2pConnection = null;
        this.connectionMode = 'relay'; // Switch to relay mode
      }
    }

    // Fallback: resolve relay URL or direct host
    if (this.baseUrl === 'https://placeholder') {
      try {
        this.baseUrl = await resolveHost(this.code, this.token);
      } catch {
        // If host resolution fails, try relay
        try {
          this.baseUrl = await resolveRelayUrl(this.code);
          this.connectionMode = 'relay';
          console.log(`[BennettSDK] Using relay: ${this.baseUrl}`);
        } catch (relayErr) {
          throw new Error(`Could not resolve host or find relay for share ${this.code}`);
        }
      }
    }
    
    this.resolved = true;

    // Initialize gRPC-Web client after resolution
    try {
      const { BennettGrpcWebClient } = await import('./grpcClient');
      this.grpcClient = new BennettGrpcWebClient({
        host: this.baseUrl.replace(/^https?:\/\//, ''),
        tls: this.baseUrl.startsWith('https'),
      });
    } catch {
      this.grpcClient = null;
    }
  }

  /**
   * Execute a SELECT query
   * PHASE 2: Routes through P2P if available, otherwise REST/gRPC
   */
  async query(sql: string, limit?: number, offset?: number): Promise<QueryResult> {
    await this.ensureResolved();

    // P2P path: send via WebRTC data channel
    if (this.p2pConnection?.isConnected()) {
      const start = performance.now();
      const response = await this.p2pConnection.send({
        type: 'query',
        sql,
        limit: limit || 1000,
        offset: offset || 0,
        token: this.token,
      });
      return {
        success: true,
        columns: response.columns || [],
        rows: response.rows || [],
        rowCount: response.rowCount || 0,
        executionTimeMs: Math.round(performance.now() - start),
        error: response.error,
      };
    }

    // REST/gRPC fallback
    const response = await this.call<QueryResult>(
      'bennett.v1.QueryService/ExecuteQuery',
      {
        shareCode: this.code,
        token: this.token,
        sql,
        limit: limit || 1000,
        offset: offset || 0,
      }
    );

    return {
      success: response.success ?? true,
      columns: response.columns || [],
      rows: response.rows || [],
      rowCount: response.rowCount || 0,
      executionTimeMs: response.executionTimeMs || 0,
      error: response.error,
    };
  }

  /**
   * Execute a write query (INSERT/UPDATE/DELETE)
   * Requires read-write permission
   */
  async write(sql: string, parameters?: any[]): Promise<WriteResult> {
    await this.ensureResolved();

    // P2P path
    if (this.p2pConnection?.isConnected()) {
      const start = performance.now();
      const response = await this.p2pConnection.send({
        type: 'write',
        sql,
        parameters: parameters || [],
        token: this.token,
      });
      return {
        success: true,
        rowsAffected: response.rowsAffected || 0,
        lastInsertId: response.lastInsertId,
        executionTimeMs: Math.round(performance.now() - start),
        error: response.error,
      };
    }

    const response = await this.call<WriteResult>(
      'bennett.v1.QueryService/ExecuteWrite',
      {
        shareCode: this.code,
        token: this.token,
        sql,
        parameters: parameters || [],
      }
    );

    return {
      success: response.success ?? true,
      rowsAffected: response.rowsAffected || 0,
      lastInsertId: response.lastInsertId,
      executionTimeMs: response.executionTimeMs || 0,
      error: response.error,
    };
  }

  /**
   * Execute query via gRPC-Web (faster for large results)
   */
  async queryGrpc(sql: string, limit?: number): Promise<QueryResult> {
    if (!this.grpcClient) {
      // Fallback to REST
      return this.query(sql, limit);
    }
    
    return this.grpcClient.query(this.code, this.token, sql, limit);
  }

  /**
   * Get schema via gRPC-Web
   */
  async getSchemaGrpc(): Promise<SchemaResult> {
    if (!this.grpcClient) {
      return this.getSchema();
    }
    
    const result = await this.grpcClient.getSchema(this.code, this.token);
    return {
      success: true,
      tables: result.tables,
      databaseName: result.databaseName,
      databaseType: result.databaseType,
      databaseVersion: '',
      error: undefined,
    };
  }

  /**
   * Get database schema
   */
  async getSchema(): Promise<SchemaResult> {
    await this.ensureResolved();

    // P2P path
    if (this.p2pConnection?.isConnected()) {
      const response = await this.p2pConnection.send({
        type: 'getSchema',
        token: this.token,
      });
      return {
        success: true,
        tables: response.tables || [],
        databaseName: response.databaseName || '',
        databaseType: response.databaseType || '',
        databaseVersion: response.databaseVersion || '',
        error: response.error,
      };
    }

    const response = await this.call<SchemaResult>(
      'bennett.v1.SchemaService/GetSchema',
      {
        shareCode: this.code,
        token: this.token,
      }
    );

    return {
      success: response.success ?? true,
      tables: response.tables || [],
      databaseName: response.databaseName || '',
      databaseType: response.databaseType || '',
      databaseVersion: response.databaseVersion || '',
      error: response.error,
    };
  }

  /**
   * Export query results as CSV
   */
  async exportCsv(sql: string, includeHeaders = true): Promise<ExportResult> {
    await this.ensureResolved();
    
    return this.call<ExportResult>(
      'bennett.v1.ExportService/ExportCsv',
      {
        shareCode: this.code,
        token: this.token,
        sql,
        format: 'csv',
        includeHeaders,
      }
    );
  }

  /**
   * Export query results as JSON
   */
  async exportJson(sql: string, includeHeaders = true): Promise<ExportResult> {
    await this.ensureResolved();
    
    return this.call<ExportResult>(
      'bennett.v1.ExportService/ExportJson',
      {
        shareCode: this.code,
        token: this.token,
        sql,
        format: 'json',
        includeHeaders,
      }
    );
  }

  /**
   * Export full table dump
   */
  async exportTable(tableName: string, format: 'csv' | 'json' = 'csv'): Promise<ExportResult> {
    await this.ensureResolved();

    return this.call<ExportResult>(
      'bennett.v1.ExportService/ExportTableDump',
      {
        shareCode: this.code,
        token: this.token,
        tableName,
        format,
      }
    );
  }

  /**
   * Create a WebSocket streaming connection for real-time queries
   * Returns a WebSocket instance — caller handles messages
   */
  createStream(baseWsUrl?: string): WebSocket {
    // Determine WebSocket URL
    // If baseUrl is http://host:port, wsUrl is ws://host:port/ws/share/CODE
    const httpUrl = this.baseUrl || baseWsUrl || 'http://localhost:8443';
    const wsProtocol = httpUrl.startsWith('https') ? 'wss' : 'ws';
    const wsHost = httpUrl.replace(/^https?:\/\//, '').replace(/\/$/, '');
    const wsUrl = `${wsProtocol}://${wsHost}/ws/share/${this.code}`;

    console.log(`[BennettClient] Opening WebSocket stream: ${wsUrl}`);

    const ws = new WebSocket(wsUrl);

    ws.onopen = () => {
      console.log('[BennettClient] WebSocket connected');
    };

    ws.onerror = (err) => {
      console.error('[BennettClient] WebSocket error:', err);
    };

    ws.onclose = () => {
      console.log('[BennettClient] WebSocket closed');
    };

    return ws;
  }

  /**
   * Execute a query via WebSocket streaming (returns async iterator)
   */
  async *queryStream(sql: string, limit?: number): AsyncGenerator<any, void, unknown> {
    const ws = this.createStream();

    // Wait for connection
    await new Promise<void>((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('WebSocket connection timeout')), 10000);
      ws.onopen = () => {
        clearTimeout(timeout);
        resolve();
      };
      ws.onerror = (err) => {
        clearTimeout(timeout);
        reject(new Error(`WebSocket connection failed: ${err}`));
      };
    });

    // Send query
    ws.send(JSON.stringify({
      type: 'query',
      sql,
      limit: limit || 1000,
      request_id: `query-${Date.now()}`,
    }));

    // Yield results as they arrive
    const messageQueue: any[] = [];
    let done = false;
    let error: Error | null = null;

    ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      if (data.type === 'query_result') {
        messageQueue.push(data);
      } else if (data.type === 'query_error') {
        error = new Error(data.error);
        done = true;
      } else if (data.type === 'status' && data.message === 'Keepalive') {
        // Ignore keepalives
      }
    };

    ws.onclose = () => {
      done = true;
    };

    // Yield messages until done
    while (!done || messageQueue.length > 0) {
      if (messageQueue.length > 0) {
        yield messageQueue.shift();
      } else {
        await new Promise(r => setTimeout(r, 10));
      }
      if (error) throw error;
    }

    ws.close();
  }

  /**
   * Convert camelCase keys to snake_case for protobuf JSON compatibility
   */
  private toSnakeCase(obj: Record<string, any>): Record<string, any> {
    const result: Record<string, any> = {};
    for (const [key, value] of Object.entries(obj)) {
      const snakeKey = key.replace(/[A-Z]/g, letter => `_${letter.toLowerCase()}`);
      result[snakeKey] = value;
    }
    return result;
  }

  /**
   * Get current connection mode for diagnostics
   */
  getConnectionMode(): 'p2p' | 'relay' | 'direct' {
    if (this.p2pConnection?.isConnected()) return 'p2p';
    return this.connectionMode;
  }

  /**
   * Close all connections (P2P + cleanup)
   */
  close(): void {
    this.p2pConnection?.close();
    this.p2pConnection = null;
  }

  /**
   * Set connection state and notify listeners
   */
  private setState(state: 'connecting' | 'connected' | 'disconnected' | 'error'): void {
    this.connectionState = state;
    for (const listener of this.stateListeners) {
      listener(state);
    }
  }

  /**
   * Subscribe to connection state changes
   */
  onStateChange(listener: (state: string) => void): () => void {
    this.stateListeners.add(listener);
    // Immediately notify current state
    listener(this.connectionState);
    return () => this.stateListeners.delete(listener);
  }

  /**
   * Low-level Connect-RPC call with auto-retry
   */
  private async call<T>(method: string, payload: Record<string, any>): Promise<T> {
    let lastError: Error | null = null;

    for (let attempt = 0; attempt < this.maxRetries; attempt++) {
      if (attempt > 0) {
        console.log(`[BennettSDK] Retry ${attempt}/${this.maxRetries} after ${this.retryDelayMs}ms...`);
        await new Promise(r => setTimeout(r, this.retryDelayMs * attempt));
      }

      try {
        this.setState('connecting');
        const result = await this.callOnce<T>(method, payload);
        this.setState('connected');
        return result;
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));
        
        // Don't retry on client errors (4xx)
        if (lastError.message.includes('HTTP 4')) {
          this.setState('error');
          throw lastError;
        }
        
        // Don't retry on auth errors
        if (lastError.message.includes('unauthorized') || lastError.message.includes('forbidden')) {
          this.setState('error');
          throw lastError;
        }
      }
    }

    this.setState('error');
    throw lastError || new Error('Request failed after max retries');
  }

  /**
   * Single attempt Connect-RPC call
   */
  private async callOnce<T>(method: string, payload: Record<string, any>): Promise<T> {
    const url = `${this.baseUrl}/${method}`;
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await fetch(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Accept': 'application/json',
        },
        body: JSON.stringify(payload),
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();

      // Check for Connect-RPC error
      if (data.code && data.message) {
        throw new Error(`Connect-RPC ${data.code}: ${data.message}`);
      }

      return data as T;
    } catch (error) {
      clearTimeout(timeoutId);

      if (error instanceof Error) {
        if (error.name === 'AbortError') {
          throw new Error('Request timeout');
        }
        throw error;
      }

      throw new Error('Unknown error');
    }
  }
}

/**
 * Decode JWT payload without verification (for extracting connection info)
 */
export function decodeJwtPayload(token: string): any {
  try {
    const parts = token.split('.');
    if (parts.length !== 3) return null;
    // Base64Url decode
    const base64 = parts[1].replace(/-/g, '+').replace(/_/g, '/');
    const padded = base64.padEnd(base64.length + (4 - base64.length % 4) % 4, '=');
    const json = atob(padded);
    return JSON.parse(json);
  } catch {
    return null;
  }
}

/**
 * Connection mode: how the client connects to the host
 */
export type ConnectionMode = 'direct' | 'p2p' | 'relay' | 'unknown';

/**
 * Connection info extracted from JWT
 */
export interface ConnectionInfo {
  mode: ConnectionMode;
  host?: string;
  port?: number;
  ice?: string;
  code: string;
  dbId: string;
  permission: string;
  tables: string[];
  expiresAt: number;
}

/**
 * Convenience function to create client from share URL
 * Automatically detects connection mode from embedded JWT
 */
export function clientFromUrl(url: string): BennettShareClient {
  // Parse https://share.bennett.studio/db/ACQPFDAQ7P?t=eyJhbG...
  const codeMatch = url.match(/\/db\/([A-Z0-9]+)/i);
  const tokenMatch = url.match(/[?&]t=([^&]+)/);

  if (!codeMatch || !tokenMatch) {
    throw new Error('Invalid share URL format. Expected: https://host/db/CODE?t=JWT');
  }

  const code = codeMatch[1].toUpperCase();
  const token = decodeURIComponent(tokenMatch[1]);

  // Decode JWT to extract connection info
  const claims = decodeJwtPayload(token);
  if (!claims) {
    console.warn('Could not decode JWT payload — falling back to direct mode');
    return new BennettShareClient({ code, token });
  }

  // Determine connection mode from JWT contents
  const hasIce = !!claims.ice;
  const hasHost = !!claims.host && !!claims.port;

  let mode: ConnectionMode = 'unknown';
  if (hasIce) {
    mode = 'p2p'; // P2P via ICE candidates (QUIC tunnel)
  } else if (hasHost) {
    mode = 'direct'; // Direct HTTP to host:port
  }

  console.log(`[BennettClient] Connection mode: ${mode}`, {
    code,
    dbId: claims.db_id,
    host: claims.host,
    port: claims.port,
    hasIce,
    permission: claims.perm,
    expiresAt: new Date(claims.exp * 1000).toISOString(),
  });

  // Build base URL based on connection mode
  let baseUrl: string | undefined;
  if (mode === 'direct' && claims.host && claims.port) {
    baseUrl = `http://${claims.host}:${claims.port}`;
  }
  // For P2P mode, baseUrl will be resolved later via relay or direct QUIC

  return new BennettShareClient({
    code,
    token,
    baseUrl,
  });
}

/**
 * Extract connection info from a share URL without creating a client
 */
export function extractConnectionInfo(url: string): ConnectionInfo | null {
  const codeMatch = url.match(/\/db\/([A-Z0-9]+)/i);
  const tokenMatch = url.match(/[?&]t=([^&]+)/);
  if (!codeMatch || !tokenMatch) return null;

  const claims = decodeJwtPayload(decodeURIComponent(tokenMatch[1]));
  if (!claims) return null;

  const hasIce = !!claims.ice;
  const hasHost = !!claims.host && !!claims.port;

  return {
    mode: hasIce ? 'p2p' : (hasHost ? 'direct' : 'unknown'),
    host: claims.host,
    port: claims.port,
    ice: claims.ice,
    code: codeMatch[1].toUpperCase(),
    dbId: claims.db_id,
    permission: claims.perm,
    tables: claims.tables || ['*'],
    expiresAt: claims.exp,
  };
}

/**
 * Get Firebase URL from environment or default
 */
function getFirebaseUrl(): string {
  if (typeof process !== 'undefined' && process.env?.BENNETT_FIREBASE_URL) {
    return process.env.BENNETT_FIREBASE_URL;
  }
  // Default Firebase RTDB for signaling
  return 'https://bennett-p2p-signaling-default-rtdb.europe-west1.firebasedatabase.app/';
}

/**
 * Convenience function to create client from code + token
 */
export function createClient(code: string, token: string, baseUrl?: string): BennettShareClient {
  return new BennettShareClient({ code, token, baseUrl });
}
