import { API_BASE_URL } from './api';
import type {
  CreateApiKeyRequest,
  CreateApiKeyResponse,
  ApiKeyInfo,
  ListApiKeysResponse,
} from '@bennettstudio/shared';

export type { CreateApiKeyRequest, CreateApiKeyResponse, ApiKeyInfo, ListApiKeysResponse };

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

  deleteApiKey: async (id: string): Promise<boolean> => {
    const response = await fetch(`${API_BASE_URL}/api/keys/${id}/permanent`, { method: 'DELETE' });
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    const result = await response.json();
    return result.success;
  },
};
