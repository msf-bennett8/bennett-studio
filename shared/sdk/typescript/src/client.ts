/**
 * Bennett Studio Connect-RPC Client SDK
 * TypeScript client for querying shared databases
 * 
 * Usage:
 *   const client = new BennettShareClient({ code: 'ACQPFDAQ7P', token: 'eyJ...' });
 *   const result = await client.query('SELECT * FROM users LIMIT 10');
 */

import { resolveHost, preloadHosts } from './resolver';

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

  constructor(config: BennettClientConfig) {
    this.code = config.code;
    this.token = config.token;
    this.baseUrl = config.baseUrl || 'https://placeholder';
    this.timeout = config.timeout || 30000;
    this.resolved = !!config.baseUrl;
    
    // Initialize gRPC-Web client if host supports it
    if (config.baseUrl) {
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
  
  /** Ensure base URL is resolved before making requests */
  private async ensureResolved(): Promise<void> {
    if (this.resolved) return;
    
    this.baseUrl = await resolveHost(this.code);
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
   */
  async query(sql: string, limit?: number, offset?: number): Promise<QueryResult> {
    await this.ensureResolved();
    
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
   * Low-level Connect-RPC call
   */
  private async call<T>(method: string, payload: Record<string, any>): Promise<T> {
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
 * Convenience function to create client from share URL
 */
export function clientFromUrl(url: string): BennettShareClient {
  // Parse https://share.bennett.studio/db/ACQPFDAQ7P?t=eyJhbG...
  const codeMatch = url.match(/\/db\/([A-Z0-9]+)/);
  const tokenMatch = url.match(/[?&]t=([^&]+)/);
  
  if (!codeMatch || !tokenMatch) {
    throw new Error('Invalid share URL format');
  }
  
  return new BennettShareClient({
    code: codeMatch[1],
    token: decodeURIComponent(tokenMatch[1]),
  });
}

/**
 * Convenience function to create client from code + token
 */
export function createClient(code: string, token: string, baseUrl?: string): BennettShareClient {
  return new BennettShareClient({ code, token, baseUrl });
}
