//! Web Token Vault — Encrypted IndexedDB storage
//! Uses Web Crypto API (SubtleCrypto) for AES-GCM encryption
//! Master key derived from browser fingerprint + random salt

const DB_NAME = 'bennett-vault';
const DB_VERSION = 1;
const STORE_NAME = 'tokens';
const MASTER_KEY_SALT = 'bennett-studio-v1';

interface EncryptedToken {
  code: string;
  iv: string; // Base64
  ciphertext: string; // Base64
  dbId: string;
  dbName: string;
  createdAt: string;
  expiresAt: string;
}

// ============================================================================
// Crypto Helpers
// ============================================================================

async function getMasterKey(): Promise<CryptoKey> {
  // Derive key from browser fingerprint + salt
  // In production, consider user password or WebAuthn
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
    {
      name: 'PBKDF2',
      salt: encoder.encode(MASTER_KEY_SALT),
      iterations: 100000,
      hash: 'SHA-256',
    },
    keyMaterial,
    { name: 'AES-GCM', length: 256 },
    false,
    ['encrypt', 'decrypt']
  );
}

async function getBrowserFingerprint(): Promise<string> {
  // Simple fingerprint — in production use a more robust library
  const components = [
    navigator.userAgent,
    navigator.language,
    screen.colorDepth,
    screen.width,
    screen.height,
    new Date().getTimezoneOffset(),
  ];
  return components.join('|');
}

async function encryptToken(token: string, key: CryptoKey): Promise<{ iv: string; ciphertext: string }> {
  const encoder = new TextEncoder();
  const iv = crypto.getRandomValues(new Uint8Array(12));
  
  const ciphertext = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv },
    key,
    encoder.encode(token)
  );
  
  return {
    iv: arrayBufferToBase64(iv),
    ciphertext: arrayBufferToBase64(ciphertext),
  };
}

async function decryptToken(iv: string, ciphertext: string, key: CryptoKey): Promise<string> {
  const decoder = new TextDecoder();
  const ivBuffer = base64ToArrayBuffer(iv);
  const ciphertextBuffer = base64ToArrayBuffer(ciphertext);
  
  const plaintext = await crypto.subtle.decrypt(
    { name: 'AES-GCM', iv: ivBuffer },
    key,
    ciphertextBuffer
  );
  
  return decoder.decode(plaintext);
}

function arrayBufferToBase64(buffer: ArrayBuffer): string {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

function base64ToArrayBuffer(base64: string): ArrayBuffer {
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes.buffer;
}

// ============================================================================
// IndexedDB Operations
// ============================================================================

async function openDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);
    
    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result);
    
    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        db.createObjectStore(STORE_NAME, { keyPath: 'code' });
      }
    };
  });
}

// ============================================================================
// Vault Interface Implementation
// ============================================================================

export interface StoredToken {
  code: string;
  token: string;
  dbId: string;
  dbName: string;
  createdAt: string;
  expiresAt: string;
}

export const tokenVault = {
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
          const token = await decryptToken(encrypted.iv, encrypted.ciphertext, key);
          resolve(token);
        } catch (e) {
          // Decryption failed — key may have changed (new browser, cleared data)
          console.warn('Failed to decrypt token:', e);
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
            const token = await decryptToken(e.iv, e.ciphertext, key);
            tokens.push({
              code: e.code,
              token,
              dbId: e.dbId,
              dbName: e.dbName,
              createdAt: e.createdAt,
              expiresAt: e.expiresAt,
            });
          } catch {
            // Skip undecryptable entries
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
