//! Vault Crypto — Hardened AES-GCM encryption for token storage
//! Industry best practices:
//!   - Input validation before every crypto operation
//!   - Automatic cleanup of corrupt entries
//!   - Stable key derivation (not browser fingerprint)
//!   - Graceful degradation on decryption failure

const MASTER_KEY_SALT = 'bennett-studio-v1';
const AES_GCM_TAG_SIZE = 16; // bytes
const AES_GCM_IV_SIZE = 12;  // bytes

// ============================================================================
// Stable Key Derivation (NOT browser fingerprint — too unstable)
// ============================================================================

/**
 * Derive a stable encryption key from a device-bound secret.
 * In production desktop: Tauri provides a stable device secret.
 * In browser: We use a random key stored in localStorage + a user-facing
 * recovery flow. This is NOT perfect but better than fingerprint drift.
 */
async function deriveKey(secret: string): Promise<CryptoKey> {
  const encoder = new TextEncoder();
  const keyMaterial = await crypto.subtle.importKey(
    'raw',
    encoder.encode(secret + MASTER_KEY_SALT),
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

/**
 * Get or create a stable device secret for key derivation.
 * Desktop: uses Tauri-provided stable ID.
 * Web: generates random UUID, stores in localStorage. Survives
 * browser restarts but NOT incognito mode or cache clear.
 */
export async function getDeviceSecret(): Promise<string> {
  const STORAGE_KEY = 'bennett-vault-secret';

  // Check if Tauri can provide a stable device ID
  if (typeof window !== 'undefined' && (window as any).__TAURI__?.invoke) {
    try {
      const deviceId = await (window as any).__TAURI__.invoke('get_device_id');
      if (deviceId) return deviceId;
    } catch {
      // Fall through to web method
    }
  }

  // Web fallback: localStorage-based secret
  let secret = localStorage.getItem(STORAGE_KEY);
  if (!secret) {
    // crypto.randomUUID() may not be available in all contexts
    secret = typeof crypto !== 'undefined' && 'randomUUID' in crypto
      ? crypto.randomUUID()
      : `${Date.now()}-${Math.random().toString(36).slice(2)}-${Math.random().toString(36).slice(2)}`;
    localStorage.setItem(STORAGE_KEY, secret);
  }
  return secret;
}

export async function getMasterKey(): Promise<CryptoKey> {
  const secret = await getDeviceSecret();
  return deriveKey(secret);
}

// ============================================================================
// Safe Base64 (handles malformed input gracefully)
// ============================================================================

export function arrayBufferToBase64(buffer: ArrayBuffer | Uint8Array): string {
  const bytes = new Uint8Array(buffer);
  let binary = '';
  for (let i = 0; i < bytes.byteLength; i++) {
    binary += String.fromCharCode(bytes[i]);
  }
  return btoa(binary);
}

export function base64ToArrayBuffer(base64: string): ArrayBuffer | null {
  if (!base64 || typeof base64 !== 'string') return null;
  try {
    const binary = atob(base64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return bytes.buffer;
  } catch {
    return null; // Invalid base64
  }
}

// ============================================================================
// Hardened Encrypt / Decrypt
// ============================================================================

export async function encryptToken(
  token: string,
  key: CryptoKey
): Promise<{ iv: string; ciphertext: string }> {
  const encoder = new TextEncoder();
  const iv = crypto.getRandomValues(new Uint8Array(AES_GCM_IV_SIZE));
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

export class DecryptionError extends Error {
  constructor(
    message: string,
    public readonly code: string,
    public readonly reason: 'empty_iv' | 'empty_ciphertext' | 'invalid_base64_iv' | 'invalid_base64_ct' | 'too_short' | 'wrong_key' | 'unknown'
  ) {
    super(message);
    this.name = 'DecryptionError';
  }
}

export async function decryptToken(
  code: string,
  iv: string,
  ciphertext: string,
  key: CryptoKey
): Promise<string> {
  // 1. Validate inputs
  if (!iv || iv.trim().length === 0) {
    throw new DecryptionError('IV is empty', code, 'empty_iv');
  }
  if (!ciphertext || ciphertext.trim().length === 0) {
    throw new DecryptionError('Ciphertext is empty', code, 'empty_ciphertext');
  }

  // 2. Decode base64
  const ivBuffer = base64ToArrayBuffer(iv);
  if (!ivBuffer) {
    throw new DecryptionError('IV is not valid base64', code, 'invalid_base64_iv');
  }
  if (ivBuffer.byteLength !== AES_GCM_IV_SIZE) {
    throw new DecryptionError(
      `IV length ${ivBuffer.byteLength} != expected ${AES_GCM_IV_SIZE}`,
      code,
      'invalid_base64_iv'
    );
  }

  const ciphertextBuffer = base64ToArrayBuffer(ciphertext);
  if (!ciphertextBuffer) {
    throw new DecryptionError('Ciphertext is not valid base64', code, 'invalid_base64_ct');
  }

  // 3. AES-GCM tag is 16 bytes appended to ciphertext
  if (ciphertextBuffer.byteLength < AES_GCM_TAG_SIZE) {
    throw new DecryptionError(
      `Ciphertext too small: ${ciphertextBuffer.byteLength} bytes (need >= ${AES_GCM_TAG_SIZE} for AES-GCM tag)`,
      code,
      'too_short'
    );
  }

  // 4. Decrypt
  const decoder = new TextDecoder();
  try {
    const plaintext = await crypto.subtle.decrypt(
      { name: 'AES-GCM', iv: ivBuffer },
      key,
      ciphertextBuffer
    );
    return decoder.decode(plaintext);
  } catch (e: any) {
    // Distinguish wrong-key from other errors
    const reason = e?.name === 'OperationError' ? 'wrong_key' : 'unknown';
    throw new DecryptionError(
      `Decryption failed: ${e?.message || 'unknown error'}`,
      code,
      reason
    );
  }
}

// ============================================================================
// IndexedDB Helpers
// ============================================================================

const DB_NAME = 'bennett-vault';
const DB_VERSION = 2; // Bump version to trigger schema migration
const STORE_NAME = 'tokens';

export interface EncryptedToken {
  code: string;
  iv: string;
  ciphertext: string;
  dbId: string;
  dbName: string;
  createdAt: string;
  expiresAt: string;
  /** Schema version for future migrations */
  _v: number;
}

export async function openDB(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open(DB_NAME, DB_VERSION);

    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result);

    request.onupgradeneeded = (event) => {
      const db = (event.target as IDBOpenDBRequest).result;
      if (!db.objectStoreNames.contains(STORE_NAME)) {
        const store = db.createObjectStore(STORE_NAME, { keyPath: 'code' });
        // No indexes needed for simple key-value
      } else {
        // Migration: clear old corrupt data on schema bump
        const tx = (event.target as IDBOpenDBRequest).transaction;
        if (tx) {
          const store = tx.objectStore(STORE_NAME);
          store.clear();
          console.log('[vaultCrypto] Cleared old vault data for schema v2');
        }
      }
    };
  });
}

/**
 * Delete a single corrupt entry from the vault.
 */
export async function deleteVaultEntry(code: string): Promise<void> {
  const db = await openDB();
  return new Promise((resolve, reject) => {
    const tx = db.transaction(STORE_NAME, 'readwrite');
    const store = tx.objectStore(STORE_NAME);
    const request = store.delete(code);
    request.onsuccess = () => resolve();
    request.onerror = () => reject(request.error);
  });
}
