//! Unified Token Vault — Platform-aware with lazy loading
//! Desktop: Tauri OS keychain via invoke
//! Web: Encrypted IndexedDB via Web Crypto
//!
//! NOTE: This SDK module provides the vault INTERFACE only.
//! The actual implementation is injected by the consuming app (desktop or web).
//! Use setVaultImpl() to provide the platform-specific backend.

import type { StoredToken, TokenVault, VaultStatus } from '@bennett/shared';

let vaultImpl: TokenVault | null = null;

/**
 * Inject the platform-specific vault implementation.
 * Call this once at app startup:
 *   - Desktop: setVaultImpl(vaultService)
 *   - Web: setVaultImpl(tokenVault)
 */
export function setVaultImpl(impl: TokenVault): void {
  vaultImpl = impl;
}

function getVault(): TokenVault {
  if (!vaultImpl) {
    throw new Error(
      'Vault not initialized. Call setVaultImpl() with a platform-specific vault ' +
      '(desktop: vaultService, web: tokenVault) before using the SDK.'
    );
  }
  return vaultImpl;
}

// Unified vault API — delegates to injected implementation
export const vault: TokenVault = {
  async getToken(code: string): Promise<string | null> {
    return getVault().getToken(code);
  },

  async setToken(token: StoredToken): Promise<void> {
    return getVault().setToken(token);
  },

  async removeToken(code: string): Promise<void> {
    return getVault().removeToken(code);
  },

  async listTokens(): Promise<StoredToken[]> {
    return getVault().listTokens();
  },

  async clear(): Promise<void> {
    return getVault().clear();
  },
};

export async function getVaultStatus(): Promise<VaultStatus> {
  const v = getVault();
  return v.status?.() ?? {
    available: true,
    type: 'memory',
    initialized: true,
  };
}
