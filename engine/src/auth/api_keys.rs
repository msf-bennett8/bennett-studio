//! API Key management — durable, long-lived credentials for external
//! (server-to-server) access to a database. Distinct from short-lived
//! share JWTs: shown once in plaintext at creation (Stripe/Supabase UX),
//! stored only as a sha256 hash, valid until explicitly revoked.

use sha2::{Digest, Sha256};

pub const API_KEY_PREFIX: &str = "bnt_live_";

/// Generate a new API key. Returns (plaintext_key, sha256_hash_hex).
pub fn generate_api_key() -> (String, String) {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rand::Rng::gen(&mut rng);
    let encoded = base62_encode(&bytes);
    let plaintext = format!("{}{}", API_KEY_PREFIX, encoded);
    let hash = hash_api_key(&plaintext);
    (plaintext, hash)
}

/// Hash an API key for storage/lookup. Never store plaintext.
pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Generate a human-friendly wire-protocol password (no prefix — meant to
/// look like a normal database password in a MySQL/Postgres connection
/// string, e.g. in a .env file, indistinguishable from a managed DB credential).
pub fn generate_wire_password() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 18] = rand::Rng::gen(&mut rng);
    base62_encode(&bytes)
}

/// Hash a wire-protocol password for storage/lookup. Same algorithm as
/// hash_api_key (sha256 hex) — kept as a separate name for call-site clarity.
pub fn hash_wire_password(password: &str) -> String {
    hash_api_key(password)
}

fn base62_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
    let mut num = bytes.to_vec();
    let mut out = Vec::new();
    while !num.iter().all(|&b| b == 0) {
        let mut rem = 0u32;
        for byte in num.iter_mut() {
            let cur = (rem << 8) | *byte as u32;
            *byte = (cur / 62) as u8;
            rem = cur % 62;
        }
        out.push(ALPHABET[rem as usize]);
        while num.len() > 1 && num[0] == 0 {
            num.remove(0);
        }
    }
    out.reverse();
    if out.is_empty() {
        out.push(ALPHABET[0]);
    }
    String::from_utf8(out).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_hash() {
        let (key, hash) = generate_api_key();
        assert!(key.starts_with(API_KEY_PREFIX));
        assert_eq!(hash_api_key(&key), hash);
        assert_ne!(key, hash);
    }
}
