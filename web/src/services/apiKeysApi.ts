import { API_BASE_URL } from './api';
import type {
  CreateApiKeyRequest,
  CreateApiKeyResponse,
  ApiKeyInfo,
  ListApiKeysResponse,
} from '@bennettstudio/shared';

export type { CreateApiKeyRequest, CreateApiKeyResponse, ApiKeyInfo, ListApiKeysResponse };

// Same remote-mode detection as shareApi.ts — API key generation is a
// host-only action (identical trust boundary to share creation), so it's
// blocked when the app is running against the public relay/Vercel deploy
// rather than a local engine.
const isRemoteMode = () => {
  const url = (import.meta as any).env?.VITE_API_URL || '';
  return url.includes('onrender.com') || url.includes('vercel.app') || window.location.hostname.includes('vercel.app');
};

export const apiKeysApi = {
  // Host-only — local engine required
  createApiKey: async (req: CreateApiKeyRequest): Promise<CreateApiKeyResponse> => {
    if (isRemoteMode()) throw new Error('Cannot create API keys from remote mode — connect to your local engine');
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

  // Host-only — local engine required
  listApiKeys: async (databaseId?: string): Promise<ListApiKeysResponse> => {
    if (isRemoteMode()) return { keys: [], total: 0 };
    const url = databaseId
      ? `${API_BASE_URL}/api/keys?database_id=${encodeURIComponent(databaseId)}`
      : `${API_BASE_URL}/api/keys`;
    const response = await fetch(url);
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    const result = await response.json();
    if (!result.success) throw new Error(result.error || 'Failed to list API keys');
    return result.data;
  },

  // Host-only — local engine required
  revokeApiKey: async (id: string): Promise<boolean> => {
    if (isRemoteMode()) return false;
    const response = await fetch(`${API_BASE_URL}/api/keys/${id}`, { method: 'DELETE' });
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    const result = await response.json();
    return result.success;
  },

  deleteApiKey: async (id: string): Promise<boolean> => {
    if (isRemoteMode()) return false;
    const response = await fetch(`${API_BASE_URL}/api/keys/${id}/permanent`, { method: 'DELETE' });
    if (!response.ok) throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    const result = await response.json();
    return result.success;
  },
};
