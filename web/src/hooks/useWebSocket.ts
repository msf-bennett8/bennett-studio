import { useEffect, useRef, useState, useCallback } from 'react';
import { isRemoteMode } from '../services/api';

export interface WsLogLine {
  type: 'log_line';
  database_id: string;
  line: string;
  timestamp: string;
  message_id: number;
}

export interface WsQueryResult {
  type: 'query_result';
  database_id: string;
  columns: string[];
  rows: any[][];
  row_count: number;
  execution_time_ms: number;
  message_id: number;
}

export interface WsHealthUpdate {
  type: 'health_update';
  database_id: string;
  status: string;
  uptime_seconds: number;
  message_id: number;
}

export interface WsHello {
  type: 'hello';
  session_id: string;
  server_time: string;
}

export interface WsReconnectAck {
  type: 'reconnect_ack';
  session_id: string;
  last_message_id: number;
  missed_messages: WsMessage[];
}

export type WsMessage = WsLogLine | WsQueryResult | WsHealthUpdate | WsHello | WsReconnectAck | { type: 'pong' } | { type: 'error'; message: string };

const RECONNECT_DELAY = 1000; // Start with 1s
const MAX_RECONNECT_DELAY = 30000; // Max 30s
const RECONNECT_JITTER = 0.5; // 50% jitter

export function useWebSocket(databaseId: string | null) {
  const wsRef = useRef<WebSocket | null>(null);
  const [logs, setLogs] = useState<string[]>([]);
  const [connected, setConnected] = useState(false);
  const [connecting, setConnecting] = useState(false);
  const [reconnectAttempt, setReconnectAttempt] = useState(0);
  
  // Session state for reconnection
  const sessionRef = useRef<{
    sessionId: string | null;
    lastMessageId: number;
    messageBuffer: Map<number, WsMessage>;
  }>({
    sessionId: null,
    lastMessageId: 0,
    messageBuffer: new Map(),
  });

  const connect = useCallback(() => {
    if (!databaseId || wsRef.current?.readyState === WebSocket.CONNECTING) return;
    
    setConnecting(true);
    
    const wsUrl = isRemoteMode()
      ? `wss://bennett-relay.onrender.com/ws/share/${databaseId}`
      : `ws://localhost:3001/api/databases/${databaseId}/ws`;
    const ws = new WebSocket(wsUrl);
    wsRef.current = ws;

    ws.onopen = () => {
      setConnected(true);
      setConnecting(false);
      setReconnectAttempt(0);
      
      // If we have a session ID, try to reconnect
      if (sessionRef.current.sessionId) {
        ws.send(JSON.stringify({
          type: 'reconnect',
          session_id: sessionRef.current.sessionId,
          last_message_id: sessionRef.current.lastMessageId,
        }));
      }
      
      // Start ping interval
      const interval = setInterval(() => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({ type: 'ping' }));
        }
      }, 30000);
      (ws as any)._pingInterval = interval;
    };

    ws.onmessage = (event) => {
      const msg: WsMessage = JSON.parse(event.data);
      
      // Track message ID for reconnection
      if ('message_id' in msg) {
        sessionRef.current.lastMessageId = msg.message_id;
        sessionRef.current.messageBuffer.set(msg.message_id, msg);
        
        // Acknowledge receipt
        if (ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({
            type: 'ack',
            message_id: msg.message_id,
          }));
        }
        
        // Clean old messages (keep last 100)
        if (sessionRef.current.messageBuffer.size > 100) {
          const oldest = Array.from(sessionRef.current.messageBuffer.keys()).sort((a, b) => a - b)[0];
          sessionRef.current.messageBuffer.delete(oldest);
        }
      }
      
      // Handle session initialization
      if (msg.type === 'hello') {
        sessionRef.current.sessionId = msg.session_id;
        sessionRef.current.lastMessageId = 0;
        sessionRef.current.messageBuffer.clear();
      }
      
      // Handle reconnection acknowledgment
      if (msg.type === 'reconnect_ack') {
        // Process missed messages
        for (const missed of msg.missed_messages) {
          if ('message_id' in missed) {
            sessionRef.current.messageBuffer.set(missed.message_id, missed);
          }
        }
      }
      
      if (msg.type === 'log_line') {
        setLogs(prev => [...prev.slice(-100), msg.line]);
      }
    };

    ws.onclose = (event) => {
      setConnected(false);
      setConnecting(false);
      
      if ((ws as any)._pingInterval) {
        clearInterval((ws as any)._pingInterval);
      }
      
      // Attempt reconnection unless clean close
      if (!event.wasClean && databaseId) {
        const delay = Math.min(
          RECONNECT_DELAY * Math.pow(2, reconnectAttempt),
          MAX_RECONNECT_DELAY
        );
        const jittered = delay * (1 + (Math.random() - 0.5) * RECONNECT_JITTER);
        
        setReconnectAttempt(prev => prev + 1);
        
        setTimeout(() => {
          connect();
        }, jittered);
      }
    };

    ws.onerror = (err) => {
      console.error('WebSocket error:', err);
      setConnected(false);
      setConnecting(false);
    };
  }, [databaseId, reconnectAttempt]);

  useEffect(() => {
    if (!databaseId) {
      // Cleanup
      if (wsRef.current) {
        wsRef.current.close(1000, 'Component unmounted');
        wsRef.current = null;
      }
      sessionRef.current.sessionId = null;
      sessionRef.current.lastMessageId = 0;
      sessionRef.current.messageBuffer.clear();
      setLogs([]);
      setConnected(false);
      setReconnectAttempt(0);
      return;
    }
    
    connect();
    
    return () => {
      if (wsRef.current) {
        wsRef.current.close(1000, 'Component unmounted');
      }
    };
  }, [databaseId, connect]);

  const executeQuery = useCallback((sql: string) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify({
        type: 'execute_query',
        database_id: databaseId,
        sql,
      }));
    }
  }, [databaseId]);

  const manualReconnect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close(1000, 'Manual reconnect');
    }
    setReconnectAttempt(0);
    sessionRef.current.messageBuffer.clear();
    connect();
  }, [connect]);

  return { 
    logs, 
    connected, 
    connecting,
    reconnectAttempt,
    executeQuery, 
    manualReconnect,
    ws: wsRef.current 
  };
}
