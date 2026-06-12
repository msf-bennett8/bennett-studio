import { create } from 'zustand';
import { api, DatabaseInstance, CreateDatabaseRequest } from '../services/api';

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

  // Actions
  fetchDatabases: () => Promise<void>;
  discoverLocalDatabases: () => Promise<void>;
  createDatabase: (req: CreateDatabaseRequest) => Promise<void>;
  deleteDatabase: (id: string) => Promise<void>;
  startDatabase: (id: string) => Promise<void>;
  stopDatabase: (id: string) => Promise<void>;
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

  fetchDatabases: async () => {
    set({ loading: true, error: null });
    try {
      const databases = await api.listDatabases();
      set({ databases, loading: false });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to fetch databases', loading: false });
    }
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

  clearError: () => set({ error: null }),
}));
