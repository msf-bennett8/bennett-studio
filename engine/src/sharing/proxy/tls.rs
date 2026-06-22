//! TLS certificate management for wire protocol proxy
//! Auto-generates self-signed certs per share, rotated every 24h

use rcgen::{Certificate, CertificateParams, KeyPair, SignatureAlgorithm};
use rustls::{ServerConfig, pki_types::PrivateKeyDer};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_rustls::TlsAcceptor;
use tracing::{info, warn};
use std::collections::HashMap;
use std::time::{SystemTime, Duration};

/// Certificate manager with per-share caching
pub struct CertManager {
    certs: RwLock<HashMap<String, ShareCert>>,
    ca_cert: Arc<Certificate>,
    ca_key: Arc<KeyPair>,
}

/// Certificate bundle for a share
struct ShareCert {
    cert: Arc<Certificate>,
    key: Arc<KeyPair>,
    created_at: SystemTime,
    tls_acceptor: TlsAcceptor,
}

impl CertManager {
    pub fn new() -> Self {
        // Generate CA certificate
        let mut ca_params = CertificateParams::new(vec!["bennett-studio-ca.local".to_string()]);
        ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
        
        let ca_key = KeyPair::generate().expect("Failed to generate CA key");
        let ca_cert = Certificate::from_params(ca_params, &ca_key)
            .expect("Failed to generate CA cert");
        
        info!("Generated CA certificate for wire protocol TLS");
        
        Self {
            certs: RwLock::new(HashMap::new()),
            ca_cert: Arc::new(ca_cert),
            ca_key: Arc::new(ca_key),
        }
    }
    
    /// Get or create TLS acceptor for a share
    pub async fn get_acceptor(&self, share_code: &str) -> Option<TlsAcceptor> {
        // Check cache
        {
            let certs = self.certs.read().await;
            if let Some(cert) = certs.get(share_code) {
                // Check if expired (> 24h)
                let age = SystemTime::now().duration_since(cert.created_at).unwrap_or(Duration::MAX);
                if age < Duration::from_secs(86400) {
                    return Some(cert.tls_acceptor.clone());
                }
            }
        }
        
        // Generate new cert
        self.generate_cert(share_code).await.ok()
    }
    
    /// Generate a new certificate for a share
    async fn generate_cert(&self, share_code: &str) -> Result<TlsAcceptor, String> {
        let mut params = CertificateParams::new(vec![
            format!("{}.share.bennett.studio", share_code),
            "localhost".to_string(),
            "127.0.0.1".to_string(),
        ]);
        
        // Add SANs for IP addresses
        params.subject_alt_names.push(rcgen::SanType::IpAddress(std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))));
        
        let key = KeyPair::generate().map_err(|e| format!("Key generation failed: {}", e))?;
        let cert = Certificate::from_params(params, &key)
            .map_err(|e| format!("Cert generation failed: {}", e))?;
        
        // Self-sign with CA
        let cert_pem = cert.serialize_pem_with_signer(&self.ca_cert, &self.ca_key)
            .map_err(|e| format!("Signing failed: {}", e))?;
        
        let key_pem = key.serialize_pem();
        
        // Build rustls config
        let cert_chain = rustls_pemfile::certs(&mut cert_pem.as_bytes())
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("Cert parse failed: {}", e))?;
        
        let key = rustls_pemfile::private_key(&mut key_pem.as_bytes())
            .map_err(|e| format!("Key parse failed: {}", e))?
            .ok_or_else(|| "No private key found".to_string())?;
        
        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert_chain, key)
            .map_err(|e| format!("TLS config failed: {}", e))?;
        
        let acceptor = TlsAcceptor::from(Arc::new(config));
        
        // Cache
        let mut certs = self.certs.write().await;
        certs.insert(share_code.to_string(), ShareCert {
            cert: Arc::new(cert),
            key: Arc::new(key),
            created_at: SystemTime::now(),
            tls_acceptor: acceptor.clone(),
        });
        
        info!("Generated TLS certificate for share {}", share_code);
        
        Ok(acceptor)
    }
    
    /// Export CA certificate for client trust
    pub fn ca_cert_pem(&self) -> String {
        self.ca_cert.serialize_pem().unwrap_or_default()
    }
    
    /// Cleanup expired certificates
    pub async fn cleanup(&self) {
        let mut certs = self.certs.write().await;
        let now = SystemTime::now();
        let expired: Vec<String> = certs
            .iter()
            .filter(|(_, c)| {
                now.duration_since(c.created_at).unwrap_or(Duration::ZERO) > Duration::from_secs(90000) // 25h
            })
            .map(|(k, _)| k.clone())
            .collect();
        
        for key in expired {
            certs.remove(&key);
            info!("Cleaned up expired TLS cert for {}", key);
        }
    }
}

/// Start background cert cleanup
pub fn start_cert_cleanup(cert_manager: Arc<CertManager>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(3600)); // 1h
        loop {
            interval.tick().await;
            cert_manager.cleanup().await;
        }
    });
}
