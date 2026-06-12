const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3001';

export interface DatabaseInstance {
  id: string;
  name: string;
  type: 'postgres' | 'mysql' | 'mariadb' | 'sqlite' | 'redis';
  version: string;
  status: 'running' | 'stopped' | 'starting' | 'error';
  port: number;
  size: string;
  created_at: string;
  container_id?: string;
}

export interface CreateDatabaseRequest {
  name: string;
  type: string;
  version: string;
}

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

export const api = {
  health: () => fetchApi<Record<string, any>>('/api/health'),
  listDatabases: () => fetchApi<DatabaseInstance[]>('/api/databases'),
  getDatabase: (id: string) => fetchApi<DatabaseInstance>(`/api/databases/${id}`),
  createDatabase: (req: CreateDatabaseRequest) => 
    fetchApi<DatabaseInstance>('/api/databases', { method: 'POST', body: JSON.stringify(req) }),
  updateDatabase: (id: string, updates: Partial<DatabaseInstance>) =>
    fetchApi<DatabaseInstance>(`/api/databases/${id}`, { method: 'PUT', body: JSON.stringify(updates) }),
  deleteDatabase: (id: string) =>
    fetchApi<{ deleted: boolean; id: string }>(`/api/databases/${id}`, { method: 'DELETE' }),
  startDatabase: (id: string) =>
    fetchApi<DatabaseInstance>(`/api/databases/${id}/start`, { method: 'POST' }),
  stopDatabase: (id: string) =>
    fetchApi<DatabaseInstance>(`/api/databases/${id}/stop`, { method: 'POST' }),

  executeQuery: (id: string, sql: string) =>
    fetchApi<{ columns: string[]; rows: any[][]; row_count: number }>(`/api/databases/${id}/query`, {
      method: 'POST',
      body: JSON.stringify({ sql }),
    }),

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
};
