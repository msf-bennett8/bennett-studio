import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { remoteApi } from '../services/remoteApi';
import type {
  RemoteConnection,
  RemoteQueryResult,
  RemoteQueryHistory,
  TableSchema,
  AutocompleteSuggestion,
  ValidateShareResponse,
} from '@bennett/shared';

interface RemoteConnectionState {
  // Connection management
  connections: RemoteConnection[];
  activeConnectionId: string | null;
  
  // UI state
  isJoinModalOpen: boolean;
  isConnecting: boolean;
  connectionError: string | null;
  
  // Query state
  currentSql: string;
  queryResult: RemoteQueryResult | null;
  isExecuting: boolean;
  queryError: string | null;
  
  // Schema state
  schema: TableSchema[] | null;
  schemaLoading: boolean;
  schemaError: string | null;
  
  // Actions
  openJoinModal: () => void;
  closeJoinModal: () => void;
  validateUrl: (url: string) => Promise<ValidateShareResponse>;
  connect: (url: string) => Promise<void>;
  disconnect: (connectionId: string) => void;
  setActiveConnection: (id: string | null) => void;
  
  // Query actions
  setCurrentSql: (sql: string) => void;
  executeQuery: () => Promise<void>;
  executeWrite: (sql: string) => Promise<void>;
  
  // Schema actions
  refreshSchema: () => Promise<void>;
  
  // Autocomplete
  getAutocomplete: (prefix: string) => AutocompleteSuggestion[];
  
  // History
  getQueryHistory: () => RemoteQueryHistory[];
  
  // Export
  exportResults: (format: 'csv' | 'json') => Promise<string>;
  
  reconnectAll: () => Promise<void>;
  clearError: () => void;
}

export const useRemoteConnectionStore = create<RemoteConnectionState>()(
  persist(
    (set, get) => ({
      connections: [],
      activeConnectionId: null,
      isJoinModalOpen: false,
      isConnecting: false,
      connectionError: null,
      currentSql: 'SELECT * FROM users LIMIT 10;',
      queryResult: null,
      isExecuting: false,
      queryError: null,
      schema: null,
      schemaLoading: false,
      schemaError: null,

      openJoinModal: () => set({ isJoinModalOpen: true, connectionError: null }),
      closeJoinModal: () => set({ isJoinModalOpen: false, connectionError: null }),
      
      validateUrl: async (url) => {
        return remoteApi.validateShare(url);
      },

      connect: async (url) => {
        set({ isConnecting: true, connectionError: null });
        try {
          const connection = await remoteApi.connect(url);
          // Store the original share URL for reconnection
          connection.shareUrl = url;

          // Deduplicate: replace existing connection with same share code
          set(state => {
            const filtered = state.connections.filter(c => c.code !== connection.code);
            return {
              connections: [...filtered, connection],
              activeConnectionId: connection.id,
              isConnecting: false,
              isJoinModalOpen: false,
            };
          });

          // Fetch schema immediately
          get().refreshSchema();
        } catch (err) {
          set({
            isConnecting: false,
            connectionError: err instanceof Error ? err.message : 'Connection failed',
          });
        }
      },

      reconnectAll: async () => {
        const { connections, activeConnectionId } = get();
        const disconnected = connections.filter(c => c.status === 'disconnected' && c.shareUrl);

        if (disconnected.length === 0) {
          return;
        }

        let reconnectedActiveId: string | null = activeConnectionId;

        for (const conn of disconnected) {
          try {
            const fresh = await remoteApi.connect(conn.shareUrl);
            fresh.shareUrl = conn.shareUrl;
            fresh.id = conn.id; // Preserve original ID

            // Track if this was the active connection
            if (conn.id === activeConnectionId) {
              reconnectedActiveId = fresh.id;
            }

            // Deduplicate by code during reconnect
            set(state => {
              const filtered = state.connections.filter(c => c.code !== fresh.code || c.id === conn.id);
              return {
                connections: filtered.map(c => c.id === conn.id ? fresh : c),
              };
            });
          } catch (err) {
            // Keep disconnected, will retry next time
            console.warn(`[reconnectAll] Failed to reconnect ${conn.code}:`, err);
          }
        }

        // Restore active connection ID if it was reconnected
        if (reconnectedActiveId) {
          set({ activeConnectionId: reconnectedActiveId });
          get().refreshSchema();
        }
      },

      disconnect: (connectionId) => {
        remoteApi.disconnect(connectionId);
        set(state => ({
          connections: state.connections.filter(c => c.id !== connectionId),
          activeConnectionId: state.activeConnectionId === connectionId 
            ? (state.connections.find(c => c.id !== connectionId)?.id || null)
            : state.activeConnectionId,
          schema: state.activeConnectionId === connectionId ? null : state.schema,
        }));
      },

      setActiveConnection: (id) => {
        set({ activeConnectionId: id, schema: null, queryResult: null, queryError: null });
        if (id) {
          get().refreshSchema();
        }
      },

      setCurrentSql: (sql) => set({ currentSql: sql }),

      executeQuery: async () => {
        const { activeConnectionId, currentSql, connections } = get();
        if (!activeConnectionId) return;
        
        const connection = connections.find(c => c.id === activeConnectionId);
        if (!connection) return;
        
        set({ isExecuting: true, queryError: null, queryResult: null });
        
        try {
          const result = await remoteApi.executeQuery(connection, currentSql);
          set({ queryResult: result, isExecuting: false });
        } catch (err) {
          set({
            queryError: err instanceof Error ? err.message : 'Query failed',
            isExecuting: false,
          });
        }
      },

      executeWrite: async (sql) => {
        const { activeConnectionId, connections } = get();
        if (!activeConnectionId) return;
        
        const connection = connections.find(c => c.id === activeConnectionId);
        if (!connection) return;
        
        set({ isExecuting: true, queryError: null });
        
        try {
          await remoteApi.executeWrite(connection, sql);
          set({ isExecuting: false });
          // Refresh results if we had a query
          if (get().queryResult) {
            await get().executeQuery();
          }
        } catch (err) {
          set({
            queryError: err instanceof Error ? err.message : 'Write failed',
            isExecuting: false,
          });
        }
      },

      refreshSchema: async () => {
        const { activeConnectionId, connections } = get();
        if (!activeConnectionId) return;
        
        const connection = connections.find(c => c.id === activeConnectionId);
        if (!connection) return;
        
        set({ schemaLoading: true, schemaError: null });
        
        try {
          const schema = await remoteApi.fetchSchema(connection, true);
          set({ schema, schemaLoading: false });
        } catch (err) {
          set({
            schemaError: err instanceof Error ? err.message : 'Failed to fetch schema',
            schemaLoading: false,
          });
        }
      },

      getAutocomplete: (prefix) => {
        const { activeConnectionId, connections } = get();
        if (!activeConnectionId) return [];
        
        const connection = connections.find(c => c.id === activeConnectionId);
        if (!connection) return [];
        
        return remoteApi.getAutocompleteSuggestions(connection, prefix);
      },

      getQueryHistory: () => {
        const { activeConnectionId, connections } = get();
        if (!activeConnectionId) return [];
        
        const connection = connections.find(c => c.id === activeConnectionId);
        if (!connection) return [];
        
        return remoteApi.getQueryHistory(connection);
      },

      exportResults: async (format) => {
        const { activeConnectionId, currentSql, connections } = get();
        if (!activeConnectionId) throw new Error('No active connection');
        
        const connection = connections.find(c => c.id === activeConnectionId);
        if (!connection) throw new Error('Connection not found');
        
        return remoteApi.exportQuery(connection, currentSql, format);
      },

      clearError: () => set({ connectionError: null, queryError: null, schemaError: null }),
    }),
    {
      name: 'bennett-remote-connections',
      partialize: (state) => ({
        connections: state.connections.map(c => ({
          id: c.id,
          code: c.code,
          token: c.token,
          baseUrl: c.baseUrl,
          dbId: c.dbId,
          dbName: c.dbName,
          dbType: c.dbType,
          permission: c.permission,
          tables: c.tables,
          connectedAt: c.connectedAt,
          lastActivity: c.lastActivity,
          shareUrl: c.shareUrl,
          status: 'disconnected' as const, // Reset status on reload
        })),
        activeConnectionId: state.activeConnectionId,
      }),
      
      // Auto-migrate: deduplicate connections by share code on load
      version: 1,
      migrate: (persistedState: any, version) => {
        if (version === 0 && persistedState?.connections) {
          // Deduplicate by code, keep the most recent (last in array)
          const seen = new Set<string>();
          const deduped: any[] = [];
          for (const conn of persistedState.connections) {
            if (!seen.has(conn.code)) {
              seen.add(conn.code);
              deduped.push(conn);
            }
          }
          return { ...persistedState, connections: deduped };
        }
        return persistedState as any;
      },
    }
  )
);
