const API_BASE_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000';

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
};
