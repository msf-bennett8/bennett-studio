//! Web Vault Service — Re-export from tokenVault with TokenVault interface

import { tokenVault } from './tokenVault';
import type { TokenVault, StoredToken, VaultStatus } from '@bennett/shared';

export const vaultService: TokenVault = {
  async getToken(code: string): Promise<string | null> {
    return tokenVault.getToken(code);
  },
  
  async setToken(token: StoredToken): Promise<void> {
    return tokenVault.storeToken(token);
  },
  
  async removeToken(code: string): Promise<void> {
    return tokenVault.removeToken(code);
  },
  
  async listTokens(): Promise<StoredToken[]> {
    return tokenVault.listTokens();
  },
  
  async clear(): Promise<void> {
    return tokenVault.clear();
  },
  
  async status(): Promise<VaultStatus> {
    const s = await tokenVault.status();
    return {
      available: s.available,
      type: s.type as VaultStatus['type'],
      initialized: s.initialized,
    };
  },
};
