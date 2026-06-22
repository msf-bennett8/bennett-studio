/**
 * Bennett Studio Connect-RPC Client SDK
 * TypeScript client for querying shared databases
 *
 * Usage:
 *   const client = new BennettShareClient({ code: 'ACQPFDAQ7P', token: 'eyJ...' });
 *   const result = await client.query('SELECT * FROM users LIMIT 10');
 */
export class BennettShareClient {
    code;
    token;
    baseUrl;
    timeout;
    /** gRPC-Web client for HTTP/2 streaming */
    grpcClient;
    constructor(config) {
        this.code = config.code;
        this.token = config.token;
        this.baseUrl = config.baseUrl || this.resolveBaseUrl(config.code);
        this.timeout = config.timeout || 30000;
        // Initialize gRPC-Web client if host supports it
        try {
            const { BennettGrpcWebClient } = require('./grpcClient');
            this.grpcClient = new BennettGrpcWebClient({
                host: this.baseUrl.replace(/^https?:\/\//, ''),
                tls: this.baseUrl.startsWith('https'),
            });
        }
        catch {
            // gRPC-Web not available, use REST fallback
            this.grpcClient = null;
        }
    }
    /**
     * Resolve base URL from share code
     * In production: lookup via resolver service
     * In local dev: assume localhost:3001
     */
    resolveBaseUrl(_code) {
        // TODO: Phase 1B - Implement resolver lookup
        // For now, assume local development
        if (typeof window !== 'undefined') {
            // Browser: use current host or env
            return import.meta.env?.VITE_BENNETT_HOST || 'http://localhost:3001';
        }
        // Node.js/CLI: use env or default
        return process?.env?.BENNETT_HOST || 'http://localhost:3001';
    }
    /**
     * Execute a SELECT query
     */
    async query(sql, limit, offset) {
        const response = await this.call('bennett.v1.QueryService/ExecuteQuery', {
            shareCode: this.code,
            token: this.token,
            sql,
            limit: limit || 1000,
            offset: offset || 0,
        });
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
    async write(sql, parameters) {
        const response = await this.call('bennett.v1.QueryService/ExecuteWrite', {
            shareCode: this.code,
            token: this.token,
            sql,
            parameters: parameters || [],
        });
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
    async queryGrpc(sql, limit) {
        if (!this.grpcClient) {
            // Fallback to REST
            return this.query(sql, limit);
        }
        return this.grpcClient.query(this.code, this.token, sql, limit);
    }
    /**
     * Get schema via gRPC-Web
     */
    async getSchemaGrpc() {
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
    async getSchema() {
        const response = await this.call('bennett.v1.SchemaService/GetSchema', {
            shareCode: this.code,
            token: this.token,
        });
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
    async exportCsv(sql, includeHeaders = true) {
        return this.call('bennett.v1.ExportService/ExportCsv', {
            shareCode: this.code,
            token: this.token,
            sql,
            format: 'csv',
            includeHeaders,
        });
    }
    /**
     * Export query results as JSON
     */
    async exportJson(sql, includeHeaders = true) {
        return this.call('bennett.v1.ExportService/ExportJson', {
            shareCode: this.code,
            token: this.token,
            sql,
            format: 'json',
            includeHeaders,
        });
    }
    /**
     * Export full table dump
     */
    async exportTable(tableName, format = 'csv') {
        return this.call('bennett.v1.ExportService/ExportTableDump', {
            shareCode: this.code,
            token: this.token,
            tableName,
            format,
        });
    }
    /**
     * Low-level Connect-RPC call
     */
    async call(method, payload) {
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
            return data;
        }
        catch (error) {
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
export function clientFromUrl(url) {
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
export function createClient(code, token, baseUrl) {
    return new BennettShareClient({ code, token, baseUrl });
}
