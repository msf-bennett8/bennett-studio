import { useEffect, useRef, useState, useCallback } from 'react';

export interface WsLogLine {
  type: 'log_line';
  database_id: string;
  line: string;
  timestamp: string;
}

export interface WsQueryResult {
  type: 'query_result';
  database_id: string;
  columns: string[];
  rows: any[][];
  row_count: number;
  execution_time_ms: number;
}

export interface WsHealthUpdate {
  type: 'health_update';
  database_id: string;
  status: string;
  uptime_seconds: number;
}

export type WsMessage = WsLogLine | WsQueryResult | WsHealthUpdate | { type: 'pong' } | { type: 'error'; message: string };

export function useWebSocket(databaseId: string | null) {
  const wsRef = useRef<WebSocket | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    if (!databaseId) return;

    const ws = new WebSocket(`ws://localhost:3001/api/databases/${databaseId}/ws`);
    wsRef.current = ws;

    ws.onopen = () => {
      setConnected(true);
      const interval = setInterval(() => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({ type: 'ping' }));
        }
      }, 30000);
      (ws as any)._pingInterval = interval;
    };

    ws.onmessage = (event) => {
      const msg: WsMessage = JSON.parse(event.data);
      if (msg.type === 'log_line') {
        setLogs(prev => [...prev.slice(-100), msg.line]);
      }
    };

    ws.onclose = () => {
      setConnected(false);
      if ((ws as any)._pingInterval) {
        clearInterval((ws as any)._pingInterval);
      }
    };

    ws.onerror = (err) => {
      console.error('WebSocket error:', err);
      setConnected(false);
    };

    return () => {
      ws.close();
      if ((ws as any)._pingInterval) {
        clearInterval((ws as any)._pingInterval);
      }
    };
  }, [databaseId]);

  const executeQuery = useCallback((sql: string) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({
        type: 'execute_query',
        database_id: databaseId,
        sql,
      }));
    }
  }, [databaseId]);

  return { logs, connected, executeQuery, ws: wsRef.current };
}
