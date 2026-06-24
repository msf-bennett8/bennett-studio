//! Unified Token Vault — Auto-detects platform and uses appropriate backend
//! Desktop: Tauri OS keychain via invoke
//! Web: Encrypted IndexedDB via Web Crypto

import type { StoredToken, TokenVault, VaultStatus } from '@bennett/shared';

// Platform detection
const isTauri = () => {
  return typeof window !== 'undefined' && 
    (window as any).__TAURI__ !== undefined;
};

// Lazy-loaded vault implementation
let vaultInstance: TokenVault | null = null;

async function getVault(): Promise<TokenVault> {
  if (vaultInstance) return vaultInstance;
  
  if (isTauri()) {
    // Desktop — use Tauri secure storage
    const { vaultService } = await import('./vaultDesktop');
    vaultInstance = vaultService;
  } else {
    // Web — use encrypted IndexedDB
    const { tokenVault } = await import('./vaultWeb');
    vaultInstance = tokenVault;
  }
  
  return vaultInstance;
}

// Unified vault API
export const vault: TokenVault = {
  async getToken(code: string): Promise<string | null> {
    return (await getVault()).getToken(code);
  },
  
  async setToken(token: StoredToken): Promise<void> {
    return (await getVault()).setToken(token);
  },
  
  async removeToken(code: string): Promise<void> {
    return (await getVault()).removeToken(code);
  },
  
  async listTokens(): Promise<StoredToken[]> {
    return (await getVault()).listTokens();
  },
  
  async clear(): Promise<void> {
    return (await getVault()).clear();
  },
};

export async function getVaultStatus(): Promise<VaultStatus> {
  return (await getVault()).status?.() ?? {
    available: true,
    type: isTauri() ? 'tauri_secure' : 'indexeddb_encrypted',
    initialized: true,
  };
}
