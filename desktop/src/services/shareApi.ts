import { API_BASE_URL } from './api';
import type {
  CreateShareRequest,
  CreateShareResponse,
  ListSharesResponse,
  ValidateShareResponse,
  RevokeShareRequest,
} from '@bennettstudio/shared';

export const shareApi = {
  // Create a new share link
  createShare: async (req: CreateShareRequest): Promise<CreateShareResponse> => {
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

  // List all active shares
  listShares: async (): Promise<ListSharesResponse> => {
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

    // Toggle pin status for a share
  togglePin: async (code: string): Promise<boolean> => {
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
  
  // Hard delete a share (permanent removal)
  deleteShare: async (code: string): Promise<boolean> => {
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

  // Revoke a share
  revokeShare: async (code: string, reason?: string): Promise<boolean> => {
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
    const response = await fetch(`${API_BASE_URL}/api/shares/${code}/validate`, {
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

  // Create share with P2P ICE candidates
  createShareWithIce: async (req: CreateShareRequest): Promise<CreateShareResponse & { ice?: string }> => {
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
    const response = await fetch(`${API_BASE_URL}/api/shares/${code}`);
    
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
