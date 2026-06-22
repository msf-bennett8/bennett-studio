import { create } from 'zustand';
import { shareApi } from '../services/shareApi';
import type { ShareLink, CreateShareRequest } from '@bennett/shared';

interface ShareState {
  shares: ShareLink[];
  loading: boolean;
  error: string | null;
  creating: boolean;

  fetchShares: () => Promise<void>;
  createShare: (req: CreateShareRequest) => Promise<ShareLink | null>;
  revokeShare: (code: string) => Promise<boolean>;
  clearError: () => void;
}

export const useShareStore = create<ShareState>((set, get) => ({
  shares: [],
  loading: false,
  error: null,
  creating: false,

  fetchShares: async () => {
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
      await get().fetchShares(); // Refresh list
      set({ creating: false });
      
      // Build full ShareLink from response
      const newShare: ShareLink = {
        code: result.code,
        url: result.url,
        db_id: req.database_id,
        db_name: '', // Will be filled by fetch
        db_type: '',
        permission: req.permission || 'ro',
        tables: req.tables || ['*'],
        expires_at: result.expires_at,
        created_at: new Date().toISOString(),
        guest_count: 0,
        status: 'active',
      };
      
      return newShare;
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Failed to create share';
      set({ error: msg, creating: false });
      return null;
    }
  },

  revokeShare: async (code) => {
    try {
      const success = await shareApi.revokeShare(code, 'host_revoked');
      if (success) {
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

  clearError: () => set({ error: null }),
}));
