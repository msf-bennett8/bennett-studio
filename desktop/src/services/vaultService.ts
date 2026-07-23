//! Desktop Vault Service — Unified vault with Tauri + Web fallback
//! Inside Tauri app: OS keychain via invoke
//! In browser (dev): IndexedDB + hardened Web Crypto (shared SDK)

import type { StoredToken, TokenVault, VaultStatus } from '@bennettstudio/shared';
import {
  getMasterKey,
  encryptToken,
  decryptToken,
  DecryptionError,
  openDB,
  deleteVaultEntry,
  type EncryptedToken,
} from '@bennettstudio/sdk';

// ============================================================================
// Platform Detection
// ============================================================================

const isTauri = (): boolean => {
  return typeof window !== 'undefined' && (window as any).__TAURI__?.invoke !== undefined;
};

// ============================================================================
// Tauri Keychain Vault (production desktop app)
// ============================================================================

interface VaultToken {
  code: string;
  token: string;
  db_id: string;
  db_name: string;
  created_at: string;
  expires_at: string;
}

const tauriInvoke = <T>(cmd: string, args?: Record<string, unknown>): Promise<T> => {
  const tauri = (window as any).__TAURI__;
  return tauri.invoke(cmd, args);
};

const tauriVault: TokenVault = {
  async getToken(code: string): Promise<string | null> {
    return tauriInvoke<string | null>('vault_get_token', { code });
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
    await tauriInvoke('vault_store_token', { entry });
  },

  async removeToken(code: string): Promise<void> {
    await tauriInvoke('vault_remove_token', { code });
  },

  async listTokens(): Promise<StoredToken[]> {
    return [];
  },

  async clear(): Promise<void> {
    console.warn('Clear not supported for OS keychain vault');
  },

  async status(): Promise<VaultStatus> {
    return tauriInvoke('vault_status');
  },
};

// ============================================================================
// Web Crypto Vault (browser fallback — uses SHARED SDK crypto)
// ============================================================================

const STORE_NAME = 'tokens';

const webVault: TokenVault = {
  async getToken(code: string): Promise<string | null> {
    const db = await openDB();

    return new Promise(async (resolve, reject) => {
      const tx = db.transaction(STORE_NAME, 'readonly');
      const store = tx.objectStore(STORE_NAME);
      const request = store.get(code);

      request.onsuccess = async () => {
        const encrypted: EncryptedToken | undefined = request.result;
        if (!encrypted) {
          resolve(null);
          return;
        }

        try {
          const key = await getMasterKey();
          const token = await decryptToken(code, encrypted.iv, encrypted.ciphertext, key);
          resolve(token);
        } catch (e) {
          if (e instanceof DecryptionError) {
            console.warn(`[vaultService] ${(e as any).reason} for ${code}: ${(e as any).message}`);
          } else {
            console.warn(`[vaultService] Decryption failed for ${code}:`, e);
          }

          // Delete corrupt entry so it doesn't keep failing
          try {
            await deleteVaultEntry(code);
            console.log(`[vaultService] Deleted corrupt vault entry for ${code}`);
          } catch (delErr) {
            console.warn(`[vaultService] Failed to delete corrupt entry for ${code}:`, delErr);
          }

          resolve(null);
        }
      };

      request.onerror = () => reject(request.error);
    });
  },

  async setToken(token: StoredToken): Promise<void> {
    const db = await openDB();
    const key = await getMasterKey();
    const { iv, ciphertext } = await encryptToken(token.token, key);

    const encrypted: EncryptedToken = {
      code: token.code,
      iv,
      ciphertext,
      dbId: token.dbId,
      dbName: token.dbName,
      createdAt: token.createdAt,
      expiresAt: token.expiresAt,
      _v: 2,
    };

    return new Promise((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, 'readwrite');
      const store = tx.objectStore(STORE_NAME);
      const request = store.put(encrypted);
      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  },

  async removeToken(code: string): Promise<void> {
    const db = await openDB();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, 'readwrite');
      const store = tx.objectStore(STORE_NAME);
      const request = store.delete(code);
      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  },

  async listTokens(): Promise<StoredToken[]> {
    const db = await openDB();
    const key = await getMasterKey();

    return new Promise((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, 'readonly');
      const store = tx.objectStore(STORE_NAME);
      const request = store.getAll();

      request.onsuccess = async () => {
        const encrypted: EncryptedToken[] = request.result;
        const tokens: StoredToken[] = [];

        for (const e of encrypted) {
          try {
            const token = await decryptToken(e.code, e.iv, e.ciphertext, key);
            tokens.push({
              code: e.code,
              token,
              dbId: e.dbId,
              dbName: e.dbName,
              createdAt: e.createdAt,
              expiresAt: e.expiresAt,
            });
          } catch (e) {
            if (e instanceof DecryptionError) {
              console.warn(`[vaultService] Skipping corrupt entry ${(e as any).code}: ${(e as any).reason}`);
              try {
                await deleteVaultEntry((e as any).code);
              } catch { /* ignore cleanup error */ }
            }
          }
        }

        resolve(tokens);
      };

      request.onerror = () => reject(request.error);
    });
  },

  async clear(): Promise<void> {
    const db = await openDB();
    return new Promise((resolve, reject) => {
      const tx = db.transaction(STORE_NAME, 'readwrite');
      const store = tx.objectStore(STORE_NAME);
      const request = store.clear();
      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  },

  async status(): Promise<VaultStatus> {
    try {
      await openDB();
      return { available: true, type: 'indexeddb_encrypted', initialized: true };
    } catch {
      return { available: false, type: 'indexeddb_encrypted', initialized: false };
    }
  },
};

// ============================================================================
// Unified Vault — Auto-detects platform
// ============================================================================

function getVault(): TokenVault {
  return isTauri() ? tauriVault : webVault;
}

export const vaultService: TokenVault = {
  async getToken(code: string): Promise<string | null> {
    try {
      return await getVault().getToken(code);
    } catch (e) {
      console.warn('[vaultService] getToken failed:', e);
      return null;
    }
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

  async status(): Promise<VaultStatus> {
    try {
      const v = getVault();
      return v.status ? await v.status() : { available: true, type: isTauri() ? 'tauri_secure' : 'indexeddb_encrypted', initialized: true };
    } catch {
      return { available: false, type: isTauri() ? 'tauri_secure' : 'indexeddb_encrypted', initialized: false };
    }
  },
};

export async function getVaultStatus(): Promise<VaultStatus> {
  return vaultService.status ? await vaultService.status() : { available: true, type: isTauri() ? 'tauri_secure' : 'indexeddb_encrypted', initialized: true };
}
