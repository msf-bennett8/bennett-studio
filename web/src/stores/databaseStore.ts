import { create } from 'zustand';
import { api, DatabaseInstance, CreateDatabaseRequest, EnvFileSuggestion, DatabaseSource } from '../services/api';
import { useRemoteConnectionStore } from './remoteConnectionStore';

interface DatabaseState {
  databases: DatabaseInstance[];
  localDatabases: DatabaseInstance[];
  loading: boolean;
  error: string | null;
  selectedDatabase: DatabaseInstance | null;
  selectedTable: string | null;
  tableData: {
    columns: string[];
    rows: any[][];
    row_count: number;
    total_count: number;
  } | null;
  tableDataLoading: boolean;
  editingRow: any | null;
  logs: string[];
  unlockedDatabases: Set<string>;
  envSuggestions: Record<string, EnvFileSuggestion[]>;

  // Actions
  fetchDatabases: () => Promise<void>;
  discoverLocalDatabases: () => Promise<void>;
  createDatabase: (req: CreateDatabaseRequest) => Promise<void>;
  deleteDatabase: (id: string) => Promise<void>;
  startDatabase: (id: string) => Promise<void>;
  stopDatabase: (id: string) => Promise<void>;
  unlockDatabase: (id: string, username: string, password: string, database: string) => Promise<boolean>;
  getDatabaseStatus: (id: string) => Promise<void>;
  scanEnvFiles: (id: string) => Promise<void>;
  selectDatabase: (db: DatabaseInstance | null) => void;
  selectTable: (table: string | null) => void;
  setEditingRow: (row: any | null) => void;
  clearEditingRow: () => void;
  fetchTableData: (dbId: string, table: string, options?: {
    limit?: number;
    offset?: number;
    order_by?: string;
    order_dir?: 'ASC' | 'DESC';
    filter?: string;
  }) => Promise<void>;
  updateRow: (dbId: string, table: string, primaryKey: any, primaryKeyColumn: string, data: Record<string, any>) => Promise<void>;
  deleteRow: (dbId: string, table: string, primaryKey: any, primaryKeyColumn: string) => Promise<void>;
  insertRow: (dbId: string, table: string, data: Record<string, any>) => Promise<void>;
  clearError: () => void;
}

export const useDatabaseStore = create<DatabaseState>((set, get) => ({
  databases: [],
  localDatabases: [],
  loading: false,
  error: null,
  selectedDatabase: null,
  selectedTable: null,
  tableData: null,
  tableDataLoading: false,
  editingRow: null,
  logs: [],
  unlockedDatabases: new Set(),
  envSuggestions: {},

  fetchDatabases: async () => {
    set({ loading: true, error: null });
    try {
      const databases = await api.listDatabases();
      set({ databases, loading: false });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to fetch databases', loading: false });
    }
  },

  getRemoteDatabases: () => {
    const { connections } = useRemoteConnectionStore.getState();
    return connections
      .filter(c => c.status === 'connected')
      .map(c => ({
        id: c.id,
        name: `${c.dbName || c.code} (Remote)`,
        type: (c.dbType || 'postgres') as 'postgres' | 'mysql' | 'mariadb' | 'sqlite' | 'redis',
        version: '',
        status: 'running' as const,
        port: 0,
        size: '',
        created_at: c.connectedAt,
        source: 'bennett' as DatabaseSource,
        isRemote: true,
        shareCode: c.code,
        remotePermission: c.permission,
        remoteHost: c.baseUrl,
      }));
  },

  discoverLocalDatabases: async () => {
    set({ loading: true, error: null });
    try {
      const localDatabases = await api.discoverLocalDatabases();
      set({ localDatabases, loading: false });
      const current = get().databases.filter(d => d.source !== 'local');
      const merged = [...current, ...localDatabases];
      set({ databases: merged });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to discover local databases', loading: false });
    }
  },

  createDatabase: async (req) => {
    set({ loading: true, error: null });
    try {
      await api.createDatabase(req);
      await get().fetchDatabases();
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to create database', loading: false });
    }
  },

  deleteDatabase: async (id) => {
    set({ loading: true, error: null });
    try {
      await api.deleteDatabase(id);
      await get().fetchDatabases();
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to delete database', loading: false });
    }
  },

  startDatabase: async (id) => {
    set({ loading: true, error: null });
    try {
      await api.startDatabase(id);
      await get().fetchDatabases();
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to start database', loading: false });
    }
  },

  stopDatabase: async (id) => {
    set({ loading: true, error: null });
    try {
      await api.stopDatabase(id);
      await get().fetchDatabases();
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to stop database', loading: false });
    }
  },

  selectDatabase: (db) => set({ selectedDatabase: db }),
  selectTable: (table) => set({ selectedTable: table, tableData: null, editingRow: null }),
  setEditingRow: (row) => set({ editingRow: row }),
  clearEditingRow: () => set({ editingRow: null }),

  fetchTableData: async (dbId: string, table: string, options?: {
    limit?: number;
    offset?: number;
    order_by?: string;
    order_dir?: 'ASC' | 'DESC';
    filter?: string;
  }) => {
    set({ tableDataLoading: true, error: null });

    // Check if this is a remote database
    const remoteDb = get().getRemoteDatabases().find(d => d.id === dbId);
    if (remoteDb) {
      const { connections } = useRemoteConnectionStore.getState();
      const conn = connections.find(c => c.id === dbId);
      if (!conn) {
        set({ error: 'Remote connection not found', tableDataLoading: false });
        return;
      }
      try {
        const data = await remoteApi.fetchTableData(conn, table, options);
        set({ tableData: data, tableDataLoading: false });
      } catch (err) {
        set({
          error: err instanceof Error ? err.message : 'Failed to fetch remote table data',
          tableDataLoading: false,
        });
      }
      return;
    }

    try {
      const data = await api.getTableData(dbId, {
        table,
        limit: options?.limit ?? 50,
        offset: options?.offset ?? 0,
        order_by: options?.order_by,
        order_dir: options?.order_dir,
        filter: options?.filter,
      });
      set({ tableData: data, tableDataLoading: false });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : 'Failed to fetch table data',
        tableDataLoading: false,
      });
    }
  },

  updateRow: async (dbId: string, table: string, primaryKey: any, primaryKeyColumn: string, data: Record<string, any>) => {
    set({ loading: true, error: null });
    try {
      await api.updateRow(dbId, { table, primary_key: primaryKey, primary_key_column: primaryKeyColumn, data });
      set({ loading: false, editingRow: null });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : 'Failed to update row',
        loading: false,
      });
    }
  },

  deleteRow: async (dbId: string, table: string, primaryKey: any, primaryKeyColumn: string) => {
    set({ loading: true, error: null });
    try {
      await api.deleteRow(dbId, { table, primary_key: primaryKey, primary_key_column: primaryKeyColumn });
      set({ loading: false });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : 'Failed to delete row',
        loading: false,
      });
    }
  },

  insertRow: async (dbId: string, table: string, data: Record<string, any>) => {
    set({ loading: true, error: null });
    try {
      await api.insertRow(dbId, { table, data });
      set({ loading: false });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : 'Failed to insert row',
        loading: false,
      });
    }
  },

  unlockDatabase: async (id: string, username: string, password: string, database: string) => {
    set({ loading: true, error: null });
    try {
      const result = await api.unlockDatabase(id, { username, password, database });
      if (result.is_unlocked) {
        set(state => ({
          unlockedDatabases: new Set([...state.unlockedDatabases, id]),
          loading: false,
        }));
        await get().fetchDatabases();
        return true;
      } else {
        set({ error: 'Unlock failed', loading: false });
        return false;
      }
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to unlock database';
      set({ error: msg, loading: false });
      return false;
    }
  },

  getDatabaseStatus: async (id: string) => {
    try {
      const status = await api.getDatabaseStatus(id);
      if (status.is_unlocked) {
        set(state => ({
          unlockedDatabases: new Set([...state.unlockedDatabases, id]),
        }));
      }
    } catch (err) {
      // Silently fail
    }
  },

  scanEnvFiles: async (id: string) => {
    try {
      const suggestions = await api.scanEnvFiles(id);
      set(state => ({
        envSuggestions: { ...state.envSuggestions, [id]: suggestions },
      }));
    } catch (err) {
      // Silently fail
    }
  },

  clearError: () => set({ error: null }),
}));
