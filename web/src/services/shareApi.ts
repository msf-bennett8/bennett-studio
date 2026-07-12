import { API_BASE_URL } from './api';
import type {
  CreateShareRequest,
  CreateShareResponse,
  ListSharesResponse,
  ValidateShareResponse,
  RevokeShareRequest,
} from '@bennettstudio/shared';

// Detect if we're on the web app (remote) vs local engine
const isRemoteMode = () => {
  const url = (import.meta as any).env?.VITE_API_URL || '';
  return url.includes('onrender.com') || url.includes('vercel.app') || window.location.hostname.includes('vercel.app');
};

export const shareApi = {
  // Create a new share link — local engine only
  createShare: async (req: CreateShareRequest): Promise<CreateShareResponse> => {
    if (isRemoteMode()) throw new Error('Cannot create shares from remote mode');
    const response = await fetch(`${API_BASE_URL}/api/shares`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    });
    
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    
    const result = await response.json();
    if (!result.success) {
      throw new Error(result.error || 'Failed to create share');
    }
    return result.data;
  },

  // List all active shares — local engine only
  listShares: async (): Promise<ListSharesResponse> => {
    if (isRemoteMode()) return { shares: [], total: 0 };
    const response = await fetch(`${API_BASE_URL}/api/shares`);

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const result = await response.json();
    if (!result.success) {
      throw new Error(result.error || 'Failed to list shares');
    }
    return result.data;
  },

    // Toggle pin status for a share — local engine only
  togglePin: async (code: string): Promise<boolean> => {
    if (isRemoteMode()) return false;
    const response = await fetch(`${API_BASE_URL}/api/shares/${code}/pin`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const result = await response.json();
    return result.success;
  },

  // Hard delete a share (permanent removal) — local engine only
  deleteShare: async (code: string): Promise<boolean> => {
    if (isRemoteMode()) return false;
    const response = await fetch(`${API_BASE_URL}/api/shares/${code}/permanent`, {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const result = await response.json();
    return result.success;
  },

  // Revoke a share — local engine only
  revokeShare: async (code: string, reason?: string): Promise<boolean> => {
    if (isRemoteMode()) return false;
    const response = await fetch(`${API_BASE_URL}/api/shares/${code}`, {
      method: 'DELETE',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code, reason } as RevokeShareRequest),
    });
    
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    
    const result = await response.json();
    return result.success;
  },

  // Validate a share (guest)
  validateShare: async (code: string, token: string): Promise<ValidateShareResponse> => {
    const path = isRemoteMode() ? `/api/share/${code}/validate` : `/api/shares/${code}/validate`;
    const response = await fetch(`${API_BASE_URL}${path}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ code, token }),
    });
    
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    
    const result = await response.json();
    if (!result.success) {
      throw new Error(result.error || 'Invalid share');
    }
    return result.data;
  },

  // Create share with P2P ICE candidates — local engine only
  createShareWithIce: async (req: CreateShareRequest): Promise<CreateShareResponse & { ice?: string }> => {
    if (isRemoteMode()) throw new Error('Cannot create shares from remote mode');
    const response = await fetch(`${API_BASE_URL}/api/shares`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(req),
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    const result = await response.json();
    if (!result.success) {
      throw new Error(result.error || 'Failed to create share');
    }

    // If engine returned ICE, include it
    return result.data;
  },

  // Get public share info
  getShareInfo: async (code: string): Promise<Partial<ValidateShareResponse>> => {
    const path = isRemoteMode() ? `/api/share/${code}` : `/api/shares/${code}`;
    const response = await fetch(`${API_BASE_URL}${path}`);
    
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }
    
    const result = await response.json();
    if (!result.success) {
      throw new Error(result.error || 'Share not found');
    }
    return result.data;
  },
};
