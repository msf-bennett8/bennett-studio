/**
 * Bennett Studio gRPC-Web Client SDK
 * Uses Connect-RPC protocol over HTTP/1.1 for browser compatibility
 * Falls back to HTTP/2 when available
 */
/**
 * gRPC-Web client for browser environments
 * Uses Connect-RPC protocol for maximum compatibility
 */
export class BennettGrpcWebClient {
    baseUrl;
    timeout;
    headers;
    constructor(config) {
        const protocol = config.tls ? 'https' : 'http';
        this.baseUrl = `${protocol}://${config.host}`;
        this.timeout = config.timeout || 30000;
        this.headers = {
            'Content-Type': 'application/json',
            'Connect-Protocol-Version': '1',
        };
    }
    /**
     * Execute query via gRPC-Web (Connect-RPC protocol)
     */
    async query(shareCode, token, sql, limit) {
        const response = await this.call('bennett.v1.QueryService/ExecuteQuery', {
            shareCode,
            token,
            sql,
            limit: limit || 1000,
        });
        return {
            columns: response.columns || [],
            rows: this.parseRows(response.rows, response.columns || []),
            rowCount: response.rowCount || 0,
            executionTimeMs: response.executionTimeMs || 0,
        };
    }
    /**
     * Get schema via gRPC-Web
     */
    async getSchema(shareCode, token) {
        const response = await this.call('bennett.v1.SchemaService/GetSchema', {
            shareCode,
            token,
        });
        return {
            tables: response.tables || [],
            databaseName: response.databaseName || '',
            databaseType: response.databaseType || '',
        };
    }
    /**
     * Low-level gRPC-Web call using Connect-RPC protocol
     */
    async call(method, payload) {
        const url = `${this.baseUrl}/${method}`;
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeout);
        try {
            const response = await fetch(url, {
                method: 'POST',
                headers: this.headers,
                body: JSON.stringify(payload),
                signal: controller.signal,
                // Required for gRPC-Web/CORS
                mode: 'cors',
                credentials: 'omit',
            });
            clearTimeout(timeoutId);
            if (!response.ok) {
                throw new Error(`HTTP ${response.status}: ${response.statusText}`);
            }
            // Connect-RPC returns JSON with error in body if application error
            const data = await response.json();
            // Check for gRPC error
            if (data.code) {
                throw new Error(`gRPC ${data.code}: ${data.message}`);
            }
            return data;
        }
        catch (error) {
            clearTimeout(timeoutId);
            throw error;
        }
    }
    /**
     * Parse protobuf rows to JSON
     */
    parseRows(rows, columns) {
        if (!rows || !Array.isArray(rows))
            return [];
        return rows.map((row) => {
            if (row.values) {
                // Protobuf format: { values: [{ kind: 'stringValue', stringValue: '...' }] }
                return row.values.map((v) => {
                    if (v.nullValue !== undefined)
                        return null;
                    if (v.stringValue !== undefined)
                        return v.stringValue;
                    if (v.int64Value !== undefined)
                        return Number(v.int64Value);
                    if (v.doubleValue !== undefined)
                        return v.doubleValue;
                    if (v.boolValue !== undefined)
                        return v.boolValue;
                    return v;
                });
            }
            // Already JSON format
            return row;
        });
    }
}
/**
 * Create gRPC-Web client from share URL
 */
export function createGrpcWebClient(url) {
    const parsed = new URL(url);
    const host = parsed.host; // includes port
    return new BennettGrpcWebClient({
        host,
        tls: parsed.protocol === 'https:',
    });
}
