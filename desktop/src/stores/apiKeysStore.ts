import { create } from 'zustand';
import { apiKeysApi, ApiKeyInfo, CreateApiKeyRequest } from '../services/apiKeysApi';

interface ApiKeysState {
  keys: ApiKeyInfo[];
  loading: boolean;
  creating: boolean;
  error: string | null;
  /** Plaintext key shown once right after creation — never persisted */
  justCreatedKey: string | null;
  /** Wire-protocol credentials shown once right after creation, if enabled */
  justCreatedWireCreds: { username: string; password: string } | null;

  fetchKeys: (databaseId?: string) => Promise<void>;
  createKey: (req: CreateApiKeyRequest) => Promise<boolean>;
  revokeKey: (id: string) => Promise<boolean>;
  deleteKey: (id: string) => Promise<boolean>;
  dismissJustCreatedKey: () => void;
  clearError: () => void;
}

export const useApiKeysStore = create<ApiKeysState>((set) => ({
  keys: [],
  loading: false,
  creating: false,
  error: null,
  justCreatedKey: null,
  justCreatedWireCreds: null,

  fetchKeys: async (databaseId) => {
    set({ loading: true, error: null });
    try {
      const result = await apiKeysApi.listApiKeys(databaseId);
      set({ keys: result.keys, loading: false });
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to fetch API keys';
      set({ error: msg, loading: false });
    }
  },

  createKey: async (req) => {
    set({ creating: true, error: null });
    try {
      const result = await apiKeysApi.createApiKey(req);
      set((state) => ({
        creating: false,
        justCreatedKey: result.key,
        justCreatedWireCreds: result.wire_username && result.wire_password
          ? { username: result.wire_username, password: result.wire_password }
          : null,
        keys: [
          {
            id: result.id,
            name: result.name,
            db_id: req.database_id,
            permission: result.permission,
            tables: req.tables || ['*'],
            created_at: result.created_at,
            last_used_at: null,
            revoked: false,
            key_preview: `${result.key.slice(0, 12)}...`,
            max_rows: req.max_rows ?? 1000,
            timeout_secs: req.timeout_secs ?? 30,
            wire_enabled: !!result.wire_username,
            wire_username: result.wire_username ?? null,
          },
          ...state.keys,
        ],
      }));
      return true;
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to create API key';
      set({ error: msg, creating: false });
      return false;
    }
  },

  revokeKey: async (id) => {
    try {
      const success = await apiKeysApi.revokeApiKey(id);
      if (success) {
        set((state) => ({
          keys: state.keys.map((k) => (k.id === id ? { ...k, revoked: true } : k)),
        }));
      }
      return success;
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to revoke API key';
      set({ error: msg });
      return false;
    }
  },

  deleteKey: async (id) => {
    try {
      const success = await apiKeysApi.deleteApiKey(id);
      if (success) {
        set((state) => ({
          keys: state.keys.filter((k) => k.id !== id),
        }));
      }
      return success;
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to delete API key';
      set({ error: msg });
      return false;
    }
  },

  dismissJustCreatedKey: () => set({ justCreatedKey: null, justCreatedWireCreds: null }),
  clearError: () => set({ error: null }),
}));
