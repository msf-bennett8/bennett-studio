/**
 * Bennett Studio Connect-RPC Client SDK
 * TypeScript client for querying shared databases
 *
 * Usage:
 *   const client = new BennettShareClient({ code: 'ACQPFDAQ7P', token: 'eyJ...' });
 *   const result = await client.query('SELECT * FROM users LIMIT 10');
 */
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
    data: string;
    isLast: boolean;
    totalRows: number;
    chunkIndex: number;
    error?: string;
}
export declare class BennettShareClient {
    private code;
    private token;
    private baseUrl;
    private timeout;
    /** gRPC-Web client for HTTP/2 streaming */
    private grpcClient?;
    constructor(config: BennettClientConfig);
    /**
     * Resolve base URL from share code
     * In production: lookup via resolver service
     * In local dev: assume localhost:3001
     */
    private resolveBaseUrl;
    /**
     * Execute a SELECT query
     */
    query(sql: string, limit?: number, offset?: number): Promise<QueryResult>;
    /**
     * Execute a write query (INSERT/UPDATE/DELETE)
     * Requires read-write permission
     */
    write(sql: string, parameters?: any[]): Promise<WriteResult>;
    /**
     * Execute query via gRPC-Web (faster for large results)
     */
    queryGrpc(sql: string, limit?: number): Promise<QueryResult>;
    /**
     * Get schema via gRPC-Web
     */
    getSchemaGrpc(): Promise<SchemaResult>;
    /**
     * Get database schema
     */
    getSchema(): Promise<SchemaResult>;
    /**
     * Export query results as CSV
     */
    exportCsv(sql: string, includeHeaders?: boolean): Promise<ExportResult>;
    /**
     * Export query results as JSON
     */
    exportJson(sql: string, includeHeaders?: boolean): Promise<ExportResult>;
    /**
     * Export full table dump
     */
    exportTable(tableName: string, format?: 'csv' | 'json'): Promise<ExportResult>;
    /**
     * Low-level Connect-RPC call
     */
    private call;
}
/**
 * Convenience function to create client from share URL
 */
export declare function clientFromUrl(url: string): BennettShareClient;
/**
 * Convenience function to create client from code + token
 */
export declare function createClient(code: string, token: string, baseUrl?: string): BennettShareClient;
//# sourceMappingURL=client.d.ts.map