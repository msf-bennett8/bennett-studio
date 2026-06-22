/**
 * Bennett Studio gRPC-Web Client SDK
 * Uses Connect-RPC protocol over HTTP/1.1 for browser compatibility
 * Falls back to HTTP/2 when available
 */
export interface GrpcClientConfig {
    /** Host address (e.g., 'localhost:3002') */
    host: string;
    /** Use TLS (default: false for local dev) */
    tls?: boolean;
    /** Request timeout in ms */
    timeout?: number;
    /** Use binary protobuf (default: false = JSON) */
    binary?: boolean;
}
/**
 * gRPC-Web client for browser environments
 * Uses Connect-RPC protocol for maximum compatibility
 */
export declare class BennettGrpcWebClient {
    private baseUrl;
    private timeout;
    private headers;
    constructor(config: GrpcClientConfig);
    /**
     * Execute query via gRPC-Web (Connect-RPC protocol)
     */
    query(shareCode: string, token: string, sql: string, limit?: number): Promise<{
        columns: string[];
        rows: any[][];
        rowCount: number;
        executionTimeMs: number;
    }>;
    /**
     * Get schema via gRPC-Web
     */
    getSchema(shareCode: string, token: string): Promise<{
        tables: any[];
        databaseName: string;
        databaseType: string;
    }>;
    /**
     * Low-level gRPC-Web call using Connect-RPC protocol
     */
    private call;
    /**
     * Parse protobuf rows to JSON
     */
    private parseRows;
}
/**
 * Create gRPC-Web client from share URL
 */
export declare function createGrpcWebClient(url: string): BennettGrpcWebClient;
//# sourceMappingURL=grpcClient.d.ts.map