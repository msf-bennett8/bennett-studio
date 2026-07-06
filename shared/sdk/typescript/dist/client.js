/**
 * Bennett Studio Connect-RPC Client SDK
 * TypeScript client for querying shared databases
 *
 * Usage:
 *   const client = new BennettShareClient({ code: 'ACQPFDAQ7P', token: 'eyJ...' });
 *   const result = await client.query('SELECT * FROM users LIMIT 10');
 */
import { resolveHost } from './resolver';
export class BennettShareClient {
    code;
    token;
    baseUrl;
    timeout;
    resolved;
    /** gRPC-Web client for HTTP/2 streaming */
    grpcClient;
    constructor(config) {
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
            }
            catch {
                this.grpcClient = null;
            }
        }
    }
    /** Ensure base URL is resolved before making requests */
    async ensureResolved() {
        if (this.resolved)
            return;
        this.baseUrl = await resolveHost(this.code);
        this.resolved = true;
        // Initialize gRPC-Web client after resolution
        try {
            const { BennettGrpcWebClient } = await import('./grpcClient');
            this.grpcClient = new BennettGrpcWebClient({
                host: this.baseUrl.replace(/^https?:\/\//, ''),
                tls: this.baseUrl.startsWith('https'),
            });
        }
        catch {
            this.grpcClient = null;
        }
    }
    /**
     * Execute a SELECT query
     */
    async query(sql, limit, offset) {
        await this.ensureResolved();
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
        await this.ensureResolved();
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
        await this.ensureResolved();
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
        await this.ensureResolved();
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
        await this.ensureResolved();
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
        await this.ensureResolved();
        return this.call('bennett.v1.ExportService/ExportTableDump', {
            shareCode: this.code,
            token: this.token,
            tableName,
            format,
        });
    }
    /**
     * Create a WebSocket streaming connection for real-time queries
     * Returns a WebSocket instance — caller handles messages
     */
    createStream(baseWsUrl) {
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
    async *queryStream(sql, limit) {
        const ws = this.createStream();
        // Wait for connection
        await new Promise((resolve, reject) => {
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
        const messageQueue = [];
        let done = false;
        let error = null;
        ws.onmessage = (event) => {
            const data = JSON.parse(event.data);
            if (data.type === 'query_result') {
                messageQueue.push(data);
            }
            else if (data.type === 'query_error') {
                error = new Error(data.error);
                done = true;
            }
            else if (data.type === 'status' && data.message === 'Keepalive') {
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
            }
            else {
                await new Promise(r => setTimeout(r, 10));
            }
            if (error)
                throw error;
        }
        ws.close();
    }
    /**
     * Convert camelCase keys to snake_case for protobuf JSON compatibility
     */
    toSnakeCase(obj) {
        const result = {};
        for (const [key, value] of Object.entries(obj)) {
            const snakeKey = key.replace(/[A-Z]/g, letter => `_${letter.toLowerCase()}`);
            result[snakeKey] = value;
        }
        return result;
    }
    /**
     * Low-level Connect-RPC call
     */
    async call(method, payload) {
        const url = `${this.baseUrl}/${method}`;
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeout);
        // Server accepts camelCase natively via #[serde(rename_all = "camelCase")]
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
 * Decode JWT payload without verification (for extracting connection info)
 */
function decodeJwtPayload(token) {
    try {
        const parts = token.split('.');
        if (parts.length !== 3)
            return null;
        // Base64Url decode
        const base64 = parts[1].replace(/-/g, '+').replace(/_/g, '/');
        const padded = base64.padEnd(base64.length + (4 - base64.length % 4) % 4, '=');
        const json = atob(padded);
        return JSON.parse(json);
    }
    catch {
        return null;
    }
}
/**
 * Convenience function to create client from share URL
 * Automatically detects connection mode from embedded JWT
 */
export function clientFromUrl(url) {
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
    let mode = 'unknown';
    if (hasIce) {
        mode = 'p2p'; // P2P via ICE candidates (QUIC tunnel)
    }
    else if (hasHost) {
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
    let baseUrl;
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
export function extractConnectionInfo(url) {
    const codeMatch = url.match(/\/db\/([A-Z0-9]+)/i);
    const tokenMatch = url.match(/[?&]t=([^&]+)/);
    if (!codeMatch || !tokenMatch)
        return null;
    const claims = decodeJwtPayload(decodeURIComponent(tokenMatch[1]));
    if (!claims)
        return null;
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
 * Convenience function to create client from code + token
 */
export function createClient(code, token, baseUrl) {
    return new BennettShareClient({ code, token, baseUrl });
}
