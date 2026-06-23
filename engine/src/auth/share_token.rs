//! JWT share token generation and validation
//! Uses Ed25519 signing keys auto-generated on first start
//! Token format: signed JWT with share permissions embedded

use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// JWT claims for a share token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareClaims {
    /// Subject: the Bennett share code (e.g., "ACQPFDAQ7P")
    pub sub: String,
    /// Database ID being shared
    pub db_id: String,
    /// Host machine fingerprint
    pub host_id: String,
    /// Host IP address for direct guest connection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// Host port for direct guest connection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    /// Permission level: "ro" | "rw" | "adm"
    pub perm: String,
    /// Allowed tables: ["*"] = all, or ["users", "orders"]
    #[serde(default = "default_all_tables")]
    pub tables: Vec<String>,
    /// Allowed columns per table: null = all, or {"users": ["id", "name"]}
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cols: Option<serde_json::Value>,
    /// Row-level security: null = none, or "tenant_id = 5"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rls: Option<String>,
    /// Issued at
    pub iat: i64,
    /// Expiration (24h default)
    pub exp: i64,
    /// Unique token ID for revocation
    pub jti: String,
}

fn default_all_tables() -> Vec<String> {
    vec!["*".to_string()]
}

/// Share permission level
#[derive(Debug, Clone, PartialEq)]
pub enum SharePermission {
    ReadOnly,
    ReadWrite,
    Admin,
}

impl SharePermission {
    pub fn from_str(s: &str) -> Self {
        match s {
            "rw" => Self::ReadWrite,
            "adm" => Self::Admin,
            _ => Self::ReadOnly,
        }
    }
    
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReadOnly => "ro",
            Self::ReadWrite => "rw",
            Self::Admin => "adm",
        }
    }
    
    pub fn can_write(&self) -> bool {
        matches!(self, Self::ReadWrite | Self::Admin)
    }
    
    pub fn can_admin(&self) -> bool {
        matches!(self, Self::Admin)
    }
}

/// Token generation result
#[derive(Debug, Clone, Serialize)]
pub struct ShareToken {
    pub token: String,
    pub code: String,
    pub expires_at: DateTime<Utc>,
    pub jti: String,
}

/// Token validation result
#[derive(Debug, Clone)]
pub struct ValidatedShare {
    pub code: String,
    pub db_id: String,
    pub host_id: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub permission: SharePermission,
    pub tables: Vec<String>,
    pub cols: Option<serde_json::Value>,
    pub rls: Option<String>,
    pub jti: String,
    pub expires_at: DateTime<Utc>,
}

/// Key manager for Ed25519 signing
pub struct ShareTokenManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    key_path: PathBuf,
}

impl ShareTokenManager {
    /// Initialize or load existing keys from ~/.bennett/keys/
    pub async fn new() -> anyhow::Result<Arc<RwLock<Self>>> {
        let home = dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?;
        let key_dir = home.join(".bennett").join("keys");
        let key_path = key_dir.join("engine-signing.pem");
        
        tokio::fs::create_dir_all(&key_dir).await?;
        
        let (encoding_key, decoding_key) = if key_path.exists() {
            info!("Loading existing signing key from {:?}", key_path);
            let secret = tokio::fs::read(&key_path).await?;
            let encoding = EncodingKey::from_secret(&secret);
            let decoding = DecodingKey::from_secret(&secret);
            (encoding, decoding)
        } else {
            info!("Generating new HMAC-SHA256 signing key at {:?}", key_path);
            let secret = Self::generate_secret();
            tokio::fs::write(&key_path, &secret).await?;
            let encoding = EncodingKey::from_secret(&secret);
            let decoding = DecodingKey::from_secret(&secret);
            (encoding, decoding)
        };
        
        Ok(Arc::new(RwLock::new(Self {
            encoding_key,
            decoding_key,
            key_path,
        })))
    }
    
    /// Generate a cryptographically secure random secret
    fn generate_secret() -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut secret = vec![0u8; 64];
        rand::Rng::fill(&mut rng, &mut secret[..]);
        secret
    }
    
    /// Create a new share token
    pub fn create_token(
        &self,
        code: String,
        db_id: String,
        host_id: String,
        host: Option<String>,
        port: Option<u16>,
        permission: SharePermission,
        tables: Vec<String>,
        cols: Option<serde_json::Value>,
        rls: Option<String>,
        duration_hours: i64,
    ) -> anyhow::Result<ShareToken> {
        let now = Utc::now();
        let expires = now + Duration::hours(duration_hours);
        let jti = uuid::Uuid::new_v4().to_string();
        
        let claims = ShareClaims {
            sub: code.clone(),
            db_id,
            host_id,
            host,
            port,
            perm: permission.as_str().to_string(),
            tables,
            cols,
            rls,
            iat: now.timestamp(),
            exp: expires.timestamp(),
            jti: jti.clone(),
        };
        
        let header = Header::new(Algorithm::HS256); // Use HS256 for now, upgrade to Ed25519 later
        let token = encode(&header, &claims, &self.encoding_key)?;
        
        Ok(ShareToken {
            token,
            code,
            expires_at: expires,
            jti,
        })
    }
    
    /// Validate a token string
    pub fn validate_token(&self, token: &str) -> anyhow::Result<ValidatedShare> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_required_spec_claims(&["exp", "sub", "jti"]);
        
        let decoded = decode::<ShareClaims>(token, &self.decoding_key, &validation)?;
        let claims = decoded.claims;
        
        // Check expiration (redundant with validation but explicit)
        let now = Utc::now().timestamp();
        if claims.exp < now {
            anyhow::bail!("Token expired");
        }
        
        Ok(ValidatedShare {
            code: claims.sub,
            db_id: claims.db_id,
            host_id: claims.host_id,
            host: claims.host,
            port: claims.port,
            permission: SharePermission::from_str(&claims.perm),
            tables: claims.tables,
            cols: claims.cols,
            rls: claims.rls,
            jti: claims.jti,
            expires_at: DateTime::from_timestamp(claims.exp, 0)
                .unwrap_or_else(|| Utc::now()),
        })
    }
    
    /// Extract code from token without full validation (for URL parsing)
    pub fn peek_code(token: &str) -> Option<String> {
        // Decode header/payload without verification to get the code
        let parts: Vec<&str> = token.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        
        let payload = base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, parts[1]).ok()?;
        let claims: ShareClaims = serde_json::from_slice(&payload).ok()?;
        Some(claims.sub)
    }
}

/// Parse a share URL: https://share.bennett.studio/db/ACQPFDAQ7P?t=eyJhbG...
pub fn parse_share_url(url: &str) -> Option<(String, String)> {
    // Extract code and token from URL
    // Expected: .../db/CODE?t=TOKEN
    let url = url.trim();
    
    // Find code after /db/
    let code_start = url.find("/db/")?;
    let code_end = url[code_start + 4..].find('?').unwrap_or(url.len() - code_start - 4);
    let code = url[code_start + 4..code_start + 4 + code_end].to_string();
    
    // Find token after ?t=
    let token_start = url.find("?t=")?;
    let token = url[token_start + 3..].to_string();
    
    Some((code, token))
}

/// Build a share URL
pub fn build_share_url(base_url: &str, code: &str, token: &str) -> String {
    format!("{}/db/{}?t={}", base_url.trim_end_matches('/'), code, token)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_share_url() {
        let url = "https://share.bennett.studio/db/ACQPFDAQ7P?t=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let (code, token) = parse_share_url(url).unwrap();
        assert_eq!(code, "ACQPFDAQ7P");
        assert_eq!(token, "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
    }
    
    #[test]
    fn test_build_share_url() {
        let url = build_share_url("https://share.bennett.studio", "ACQPFDAQ7P", "abc123");
        assert_eq!(url, "https://share.bennett.studio/db/ACQPFDAQ7P?t=abc123");
    }
}
