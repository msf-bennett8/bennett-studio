//! Desktop Vault Service — Unified vault with Tauri + Web fallback
//! Inside Tauri app: OS keychain via invoke
//! In browser (dev): IndexedDB + Web Crypto (same as web vault)

import type { StoredToken, TokenVault, VaultStatus } from '@bennett/shared';

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
// Web Crypto Vault (browser fallback — same as web/src/services/tokenVault.ts)
// ============================================================================

const DB_NAME = 'bennett-vault';
const DB_VERSION = 1;
const STORE_NAME = 'tokens';
const MASTER_KEY_SALT = 'bennett-studio-v1';

interface EncryptedToken {
  code: string;
  iv: string;
  ciphertext: string;
  dbId: string;
  dbName: string;
  createdAt: string;
  expiresAt: string;
}

async function getMasterKey(): Promise<CryptoKey> {
  const fingerprint = await getBrowserFingerprint();
  const encoder = new TextEncoder();
  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    encoder.encode(fingerprint + MASTER_KEY_SALT),
    { name: 'PBKDF2' },
    false,
    ['deriveKey']
  );
  return crypto.subtle.deriveKey(
    { name: 'PBKDF2', salt: encoder.encode(MASTER_KEY_SALT), iterations: 100000, hash: 'SHA-256' },
    keyMaterial,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
  );
}

async function getBrowserFingerprint(): Promise<string> {
  const components = [navigator.userAgent, navigator.language, screen.colorDepth, screen.width, screen.height, new Date().getTimezoneOffset()];
  return components.join('|');
}

async function encryptToken(token: string, key: CryptoKey): Promise<{ iv: string; ciphertext: string }> {
  const encoder = new TextEncoder();
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const ciphertext = await crypto.subtle.encrypt({ name: 'AES-GCM', iv }, key, encoder.encode(token));
  return { iv: arrayBufferToBase64(iv), ciphertext: arrayBufferToBase64(ciphertext) };
}

async function decryptToken(iv: string, ciphertext: string, key: CryptoKey): Promise<string> {
  const decoder = new TextDecoder();
  const ivBuffer = base64ToArrayBuffer(iv);
  const ciphertextBuffer = base64ToArrayBuffer(ciphertext);
  const plaintext = await crypto.subtle.decrypt({ name: 'AES-GCM', iv: ivBuffer }, key, ciphertextBuffer);
  return decoder.decode(plaintext);
}

function arrayBufferToBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) binary += String.fromCharCode(bytes[i]);
  return btoa(binary);
}

function base64ToArrayBuffer(base64: string): ArrayBuffer {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) bytes[i] = binary.charCodeAt(i);
  return bytes.buffer;
}

async function openDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);
    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result);
    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result;
      if (!db.objectStoreNames.contains(STORE_NAME)) db.createObjectStore(STORE_NAME, { keyPath: 'code' });
    };
  });
}

const webVault: TokenVault = {
  async getToken(code: string): Promise<string | null> {
    const db = await openDB();
    return new Promise(async (resolve, reject) => {
      const tx = db.transaction(STORE_NAME, 'readonly');
      const store = tx.objectStore(STORE_NAME);
      const request = store.get(code);
      request.onsuccess = async () => {
        const encrypted: EncryptedToken | undefined = request.result;
        if (!encrypted) { resolve(null); return; }
        try {
          const key = await getMasterKey();
          const token = await decryptToken(encrypted.iv, encrypted.ciphertext, key);
          resolve(token);
        } catch (e) {
          console.warn('Failed to decrypt token:', e);
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
    const encrypted: EncryptedToken = { code: token.code, iv, ciphertext, dbId: token.dbId, dbName: token.dbName, createdAt: token.createdAt, expiresAt: token.expiresAt };
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
            const token = await decryptToken(e.iv, e.ciphertext, key);
            tokens.push({ code: e.code, token, dbId: e.dbId, dbName: e.dbName, createdAt: e.createdAt, expiresAt: e.expiresAt });
          } catch { /* skip */ }
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
      console.warn('Vault getToken failed:', e);
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
      return await getVault().status();
    } catch {
      return { available: false, type: isTauri() ? 'tauri_secure' : 'indexeddb_encrypted', initialized: false };
    }
  },
};

export async function getVaultStatus(): Promise<VaultStatus> {
  return vaultService.status();
}
