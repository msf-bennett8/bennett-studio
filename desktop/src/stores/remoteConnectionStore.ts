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
          set(state => ({
            connections: [...state.connections, connection],
            activeConnectionId: connection.id,
            isConnecting: false,
            isJoinModalOpen: false,
          }));
          
          // Fetch schema immediately
          get().refreshSchema();
        } catch (err) {
          set({
            isConnecting: false,
            connectionError: err instanceof Error ? err.message : 'Connection failed',
          });
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
          ...c,
          status: 'disconnected' as const, // Reset status on reload
        })),
      }),
    }
  )
);
