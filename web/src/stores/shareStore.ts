import { create } from 'zustand';
import { shareApi } from '../services/shareApi';
import { vaultService } from '../services/vaultService';
import type { ShareLink, CreateShareRequest, StoredToken } from '@bennett/shared';

interface ShareState {
  shares: ShareLink[];
  loading: boolean;
  error: string | null;
  creating: boolean;
  vaultAvailable: boolean;

  fetchShares: () => Promise<void>;
  createShare: (req: CreateShareRequest) => Promise<ShareLink | null>;
  revokeShare: (code: string) => Promise<boolean>;
  getShareUrl: (code: string) => Promise<string | null>;
  clearError: () => void;
  initVault: () => Promise<void>;
}

export const useShareStore = create<ShareState>((set, get) => ({
  shares: [],
  loading: false,
  error: null,
  creating: false,
  vaultAvailable: false,

  initVault: async () => {
    try {
      const status = await vaultService.status();
      set({ vaultAvailable: status.available });
    } catch {
      set({ vaultAvailable: false });
    }
  },

  fetchShares: async () => {
    // Initialize vault on first fetch
    if (!get().vaultAvailable) {
      await get().initVault();
    }
    
    set({ loading: true, error: null });
    try {
      const result = await shareApi.listShares();
      set({ shares: result.shares, loading: false });
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to fetch shares';
      set({ error: msg, loading: false });
    }
  },

  createShare: async (req) => {
    set({ creating: true, error: null });
    try {
      const result = await shareApi.createShare(req);
      
      // Store token in OS keychain
      const tokenEntry: StoredToken = {
        code: result.code,
        token: result.token,
        dbId: req.database_id,
        dbName: '', // Will be filled by fetch
        createdAt: new Date().toISOString(),
        expiresAt: result.expires_at,
      };
      
      try {
        await vaultService.setToken(tokenEntry);
      } catch (e) {
        console.warn('Failed to store token in vault:', e);
        // Continue — token will be lost on reload but share works now
      }

      // Build ShareLink for UI
      const newShare: ShareLink = {
        code: result.code,
        url: result.url,
        db_id: req.database_id,
        db_name: '',
        db_type: '',
        permission: req.permission || 'ro',
        tables: req.tables || ['*'],
        expires_at: result.expires_at,
        created_at: new Date().toISOString(),
        guest_count: 0,
        status: 'active',
      };

      set(state => ({
        shares: [newShare, ...state.shares],
        creating: false,
      }));

      return newShare;
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to create share';
      set({ error: msg, creating: false });
      return null;
    }
  },

  deleteShare: async (code) => {
    try {
      const success = await shareApi.deleteShare(code);
      if (success) {
        // Remove from vault
        try {
          await vaultService.removeToken(code);
        } catch (e) {
          console.warn('Failed to remove token from vault:', e);
        }

        // Remove from state
        set(state => ({
          shares: state.shares.filter(s => s.code !== code),
        }));
      }
      return success;
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to delete share';
      set({ error: msg });
      return false;
    }
  },

  revokeShare: async (code) => {
    try {
      const success = await shareApi.revokeShare(code, 'host_revoked');
      if (success) {
        // Remove from vault
        try {
          await vaultService.removeToken(code);
        } catch (e) {
          console.warn('Failed to remove token from vault:', e);
        }
        
        set(state => ({
          shares: state.shares.map(s =>
            s.code === code ? { ...s, status: 'revoked' as const } : s
          ),
        }));
      }
      return success;
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to revoke share';
      set({ error: msg });
      return false;
    }
  },

  // Reconstruct full URL with token from vault
  getShareUrl: async (code: string) => {
    const share = get().shares.find(s => s.code === code);
    if (!share) return null;
    
    // Try vault first
    try {
      const token = await vaultService.getToken(code);
      if (token) {
        return `https://share.bennett.studio/db/${code}?t=${token}`;
      }
    } catch (e) {
      console.warn('Vault retrieval failed:', e);
    }
    
    // Fallback: return truncated URL (can't copy full link)
    return share.url;
  },

  clearError: () => set({ error: null }),
}));
