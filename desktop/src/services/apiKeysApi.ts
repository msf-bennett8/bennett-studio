import { API_BASE_URL } from './api';

export interface CreateApiKeyRequest {
  database_id: string;
  name: string;
  permission?: 'ro' | 'rw' | 'adm';
  tables?: string[];
  max_rows?: number;
  timeout_secs?: number;
}

export interface CreateApiKeyResponse {
  id: string;
  key: string; // plaintext — shown once
  name: string;
  permission: string;
  created_at: string;
}

export interface ApiKeyInfo {
  id: string;
  name: string;
  db_id: string;
  permission: string;
  tables: string[];
  created_at: string;
  last_used_at: string | null;
  revoked: boolean;
  key_preview: string;
  max_rows: number;
  timeout_secs: number;
}

export interface ListApiKeysResponse {
  keys: ApiKeyInfo[];
  total: number;
}

export const apiKeysApi = {
  createApiKey: async (req: CreateApiKeyRequest): Promise<CreateApiKeyResponse> => {
    const response = await fetch(`${API_BASE_URL}/api/keys`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    });
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    const result = await response.json();
    if (!result.success) throw new Error(result.error || 'Failed to create API key');
    return result.data;
  },

  listApiKeys: async (databaseId?: string): Promise<ListApiKeysResponse> => {
    const url = databaseId
      ? `${API_BASE_URL}/api/keys?database_id=${encodeURIComponent(databaseId)}`
      : `${API_BASE_URL}/api/keys`;
    const response = await fetch(url);
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    const result = await response.json();
    if (!result.success) throw new Error(result.error || 'Failed to list API keys');
    return result.data;
  },

  revokeApiKey: async (id: string): Promise<boolean> => {
    const response = await fetch(`${API_BASE_URL}/api/keys/${id}`, { method: 'DELETE' });
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    const result = await response.json();
    return result.success;
  },
};
