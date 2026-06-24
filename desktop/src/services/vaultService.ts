//! Desktop Vault Service — Tauri secure storage bridge
//! Calls Rust keychain commands via Tauri invoke

import { invoke } from '@tauri-apps/api/core';
import type { StoredToken, TokenVault, VaultStatus } from '@bennett/shared';

interface VaultToken {
  code: string;
  token: string;
  db_id: string;
  db_name: string;
  created_at: string;
  expires_at: string;
}

export const vaultService: TokenVault = {
  async getToken(code: string): Promise<string | null> {
    try {
      const result = await invoke<string | null>('vault_get_token', { code });
      return result;
    } catch (e) {
      console.warn('Vault getToken failed:', e);
      return null;
    }
  },
  
  async setToken(token: StoredToken): Promise<void> {
    const entry: VaultToken = {
      code: token.code,
      token: token.token,
      db_id: token.dbId,
      db_name: token.dbName,
      created_at: token.createdAt,
      expires_at: token.expiresAt,
    };
    
    await invoke('vault_store_token', { entry });
  },
  
  async removeToken(code: string): Promise<void> {
    await invoke('vault_remove_token', { code });
  },
  
  async listTokens(): Promise<StoredToken[]> {
    // keyring doesn't support listing, so we maintain a side index
    // For now, return empty — the share store handles listing via API
    return [];
  },
  
  async clear(): Promise<void> {
    // Not implemented for keyring — would need to track all codes
    console.warn('Clear not supported for OS keychain vault');
  },
};

export async function getVaultStatus(): Promise<VaultStatus> {
  return await invoke('vault_status');
}
