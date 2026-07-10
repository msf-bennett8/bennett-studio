// Detect if we're on the web app (remote) vs local engine
const isRemoteMode = () => {
  const url = (import.meta as any).env?.VITE_API_URL || '';
  return url.includes('onrender.com') || url.includes('vercel.app') || window.location.hostname.includes('vercel.app');
};

export const API_BASE_URL = (import.meta as any).env?.VITE_API_URL || 'http://localhost:3001';

import { DatabaseInstance, CreateDatabaseRequest, DatabaseSource, DatabaseStatusResponse, EnvFileSuggestion } from '@bennettstudio/shared';
export type { DatabaseInstance, CreateDatabaseRequest, DatabaseSource, DatabaseStatusResponse, EnvFileSuggestion };

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
}

class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
  }
}

async function fetchApi<T>(path: string, options?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE_URL}${path}`, {
    ...options,
    headers: {
      'Content-Type': 'application/json',
      ...options?.headers,
    },
  });

  if (!response.ok) {
    throw new ApiError(response.status, `HTTP ${response.status}: ${response.statusText}`);
  }

  const result: ApiResponse<T> = await response.json();
  
  if (!result.success) {
    throw new Error(result.error || 'Unknown API error');
  }

  return result.data as T;
}

import { Note } from '../stores/notesStore';

export interface CreateNoteRequest {
  title: string;
  content: string;
  tags?: string[];
  color?: string;
  pinned?: boolean;
}

export interface UpdateNoteRequest {
  title?: string;
  content?: string;
  tags?: string[];
  color?: string;
  pinned?: boolean;
  archived?: boolean;
}

export const api = {
  // Health check
  health: () => fetchApi<Record<string, any>>('/api/health'),

  // Databases
  listDatabases: () => fetchApi<DatabaseInstance[]>('/api/databases'),
  
  getDatabase: (id: string) => fetchApi<DatabaseInstance>(`/api/databases/${id}`),
  
  createDatabase: (req: CreateDatabaseRequest) => 
    fetchApi<DatabaseInstance>('/api/databases', {
      method: 'POST',
      body: JSON.stringify(req),
    }),
  
  updateDatabase: (id: string, updates: Partial<DatabaseInstance>) =>
    fetchApi<DatabaseInstance>(`/api/databases/${id}`, {
      method: 'PUT',
      body: JSON.stringify(updates),
    }),
  
  deleteDatabase: (id: string) =>
    fetchApi<{ deleted: boolean; id: string }>(`/api/databases/${id}`, {
      method: 'DELETE',
    }),
  
  startDatabase: (id: string) =>
    fetchApi<DatabaseInstance>(`/api/databases/${id}/start`, {
      method: 'POST',
    }),
  
  stopDatabase: (id: string) =>
    fetchApi<DatabaseInstance>(`/api/databases/${id}/stop`, {
      method: 'POST',
    }),

  // Query execution
  executeQuery: (id: string, sql: string) =>
    fetchApi<{ columns: string[]; rows: any[][]; row_count: number }>(`/api/databases/${id}/query`, {
      method: 'POST',
      body: JSON.stringify({ sql }),
    }),

  // Schema introspection
  getSchema: (id: string) =>
    fetchApi<{ name: string; columns: { name: string; data_type: string; nullable: boolean }[] }[]>(`/api/databases/${id}/schema`),

  getTableData: (id: string, req: {
    table: string;
    limit?: number;
    offset?: number;
    order_by?: string;
    order_dir?: 'ASC' | 'DESC';
    filter?: string;
  }) =>
    fetchApi<{
      columns: string[];
      rows: any[][];
      row_count: number;
      total_count: number;
    }>(`/api/databases/${id}/data`, {
      method: 'POST',
      body: JSON.stringify(req),
    }),

  updateRow: (id: string, req: {
    table: string;
    primary_key: any;
    primary_key_column: string;
    data: Record<string, any>;
  }) =>
    fetchApi<{ updated: boolean }>(`/api/databases/${id}/rows/update`, {
      method: 'POST',
      body: JSON.stringify(req),
    }),

  deleteRow: (id: string, req: {
    table: string;
    primary_key: any;
    primary_key_column: string;
  }) =>
    fetchApi<{ deleted: boolean }>(`/api/databases/${id}/rows/delete`, {
      method: 'POST',
      body: JSON.stringify(req),
    }),

  getTableColumns: (id: string, table: string) =>
    fetchApi<{
      name: string;
      data_type: string;
      nullable: boolean;
      has_default: boolean;
      is_primary_key: boolean;
      column_default: string | null;
    }[]>(`/api/databases/${id}/columns`, {
      method: 'POST',
      body: JSON.stringify({ table }),
    }),

  insertRow: (id: string, req: {
    table: string;
    data: Record<string, any>;
  }) =>
    fetchApi<{ inserted: boolean }>(`/api/databases/${id}/rows/insert`, {
      method: 'POST',
      body: JSON.stringify(req),
    }),

  discoverLocalDatabases: () =>
    fetchApi<DatabaseInstance[]>('/api/databases/discover', {
      method: 'POST',
    }),

      unlockDatabase: (id: string, req: { username: string; password: string; database: string }) =>
    fetchApi<DatabaseStatusResponse>(`/api/databases/${id}/unlock`, {
      method: 'POST',
      body: JSON.stringify(req),
    }),

  getDatabaseStatus: (id: string) =>
    fetchApi<DatabaseStatusResponse>(`/api/databases/${id}/status`),

  scanEnvFiles: (id: string) =>
    fetchApi<EnvFileSuggestion[]>(`/api/databases/${id}/env-scan`),

  // Notes API — local engine only, skip in remote share mode
  listNotes: () => {
    if (isRemoteMode()) return Promise.resolve([]);
    return fetchApi<Note[]>('/api/notes');
  },
  getNote: (id: string) => {
    if (isRemoteMode()) return Promise.resolve({} as Note);
    return fetchApi<Note>(`/api/notes/${id}`);
  },
  createNote: (req: CreateNoteRequest) => {
    if (isRemoteMode()) return Promise.resolve({} as Note);
    return fetchApi<Note>('/api/notes', { method: 'POST', body: JSON.stringify(req) });
  },
  updateNote: (id: string, req: UpdateNoteRequest) => {
    if (isRemoteMode()) return Promise.resolve({} as Note);
    return fetchApi<Note>(`/api/notes/${id}`, { method: 'PUT', body: JSON.stringify(req) });
  },
  deleteNote: (id: string) => {
    if (isRemoteMode()) return Promise.resolve(false);
    return fetchApi<boolean>(`/api/notes/${id}`, { method: 'DELETE' });
  },
  searchNotes: (query: string) => {
    if (isRemoteMode()) return Promise.resolve([]);
    return fetchApi<Note[]>(`/api/notes/search?q=${encodeURIComponent(query)}`);
  },
  syncNotes: (notes: Note[], deviceId: string) => {
    if (isRemoteMode()) return Promise.resolve([]);
    return fetchApi<Note[]>('/api/notes/sync', {
      method: 'POST',
      body: JSON.stringify({ notes, device_id: deviceId, last_sync: null }),
    });
  },
};
