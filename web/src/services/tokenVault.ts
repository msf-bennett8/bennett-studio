//! Web Token Vault — Hardened encrypted IndexedDB storage
//! Uses shared SDK crypto layer for consistency with desktop

import {
  getMasterKey,
  encryptToken,
  decryptToken,
  DecryptionError,
  openDB,
  deleteVaultEntry,
  type EncryptedToken,
} from '@bennett/sdk';

const STORE_NAME = 'tokens';

export interface StoredToken {
  code: string;
  token: string;
  dbId: string;
  dbName: string;
  createdAt: string;
  expiresAt: string;
}

export const tokenVault = {
  async setToken(entry: StoredToken): Promise<void> {
    return this.storeToken(entry);
  },

  async storeToken(entry: StoredToken): Promise<void> {
    const db = await openDB();
    const key = await getMasterKey();
    const { iv, ciphertext } = await encryptToken(entry.token, key);

    const encrypted: EncryptedToken = {
      code: entry.code,
      iv,
      ciphertext,
      dbId: entry.dbId,
      dbName: entry.dbName,
      createdAt: entry.createdAt,
      expiresAt: entry.expiresAt,
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
            console.warn(`[tokenVault] ${e.reason} for ${code}: ${e.message}`);
          } else {
            console.warn(`[tokenVault] Decryption failed for ${code}:`, e);
          }

          // Delete corrupt entry so it doesn't keep failing
          try {
            await deleteVaultEntry(code);
            console.log(`[tokenVault] Deleted corrupt vault entry for ${code}`);
          } catch (delErr) {
            console.warn(`[tokenVault] Failed to delete corrupt entry for ${code}:`, delErr);
          }

          resolve(null);
        }
      };

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
              console.warn(`[tokenVault] Skipping corrupt entry ${e.code}: ${e.reason}`);
              try {
                await deleteVaultEntry(e.code);
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

  async status(): Promise<{ available: boolean; type: string; initialized: boolean }> {
    try {
      await openDB();
      return {
        available: true,
        type: 'indexeddb_encrypted',
        initialized: true,
      };
    } catch {
      return {
        available: false,
        type: 'indexeddb_encrypted',
        initialized: false,
      };
    }
  },
};
