import { create } from 'zustand';
import { api, DatabaseInstance, CreateDatabaseRequest } from '../services/api';

interface DatabaseState {
  databases: DatabaseInstance[];
  loading: boolean;
  error: string | null;
  selectedDatabase: DatabaseInstance | null;
  
  // Actions
  fetchDatabases: () => Promise<void>;
  createDatabase: (req: CreateDatabaseRequest) => Promise<void>;
  deleteDatabase: (id: string) => Promise<void>;
  startDatabase: (id: string) => Promise<void>;
  stopDatabase: (id: string) => Promise<void>;
  selectDatabase: (db: DatabaseInstance | null) => void;
  clearError: () => void;
}

export const useDatabaseStore = create<DatabaseState>((set, get) => ({
  databases: [],
  loading: false,
  error: null,
  selectedDatabase: null,

  fetchDatabases: async () => {
    set({ loading: true, error: null });
    try {
      const databases = await api.listDatabases();
      set({ databases, loading: false });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : 'Failed to fetch databases', loading: false });
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
  clearError: () => set({ error: null }),
}));
