//! DTLS (Datagram TLS) handshake over punched UDP
//! Encrypts the UDP connection before running QUIC over it.
//!
//! For simplicity, we use QUIC's built-in TLS 1.3 instead of separate DTLS.
//! This file provides the TLS certificate generation for QUIC.

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::sync::Arc;
use tracing::info;

/// Generate a self-signed certificate for QUIC P2P connections
pub fn generate_p2p_cert() -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), DtlsError> {
    let mut params = CertificateParams::new(vec![
        "bennett-p2p.local".to_string(),
        "localhost".to_string(),
    ]).map_err(|e| DtlsError::CertGenFailed(e.to_string()))?;

    params.distinguished_name = DistinguishedName::new();
    params.distinguished_name.push(DnType::CommonName, "Bennett P2P");
    params.distinguished_name.push(DnType::OrganizationName, "Bennett Studio");

    let key_pair = KeyPair::generate().map_err(|e| DtlsError::CertGenFailed(e.to_string()))?;
    let cert = params.self_signed(&key_pair)
        .map_err(|e| DtlsError::CertGenFailed(e.to_string()))?;

    let cert_pem = cert.pem();
    let key_pem = key_pair.serialize_pem();

    let certs = rustls_pemfile::certs(&mut cert_pem.as_bytes())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| DtlsError::CertGenFailed(e.to_string()))?;

    let key = rustls_pemfile::private_key(&mut key_pem.as_bytes())
        .map_err(|e| DtlsError::CertGenFailed(e.to_string()))?
        .ok_or_else(|| DtlsError::CertGenFailed("No private key found".to_string()))?;

    info!("Generated P2P self-signed certificate for QUIC");

    Ok((certs, key))
}

/// Build rustls server config for QUIC
pub fn build_quinn_server_config() -> Result<quinn::ServerConfig, DtlsError> {
    let (certs, key) = generate_p2p_cert()?;

    let mut rustls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| DtlsError::TlsConfigFailed(e.to_string()))?;

    // ALPN for multiplexing
    rustls_config.alpn_protocols = vec![
        b"bennett-p2p".to_vec(),
    ];

    let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(rustls_config)
            .map_err(|e| DtlsError::TlsConfigFailed(e.to_string()))?
    ));

    // Transport config: keep connections alive, enable 0-RTT
    let mut transport = quinn::TransportConfig::default();
    transport.max_idle_timeout(Some(Duration::from_secs(30).try_into().unwrap()));
    transport.keep_alive_interval(Some(Duration::from_secs(5)));
    server_config.transport_config(Arc::new(transport));

    Ok(server_config)
}

/// Build rustls client config for QUIC
pub fn build_quinn_client_config() -> Result<quinn::ClientConfig, DtlsError> {
    let (certs, key) = generate_p2p_cert()?;

    let mut rustls_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipServerVerification))
        .with_client_auth_cert(certs, key)
        .map_err(|e| DtlsError::TlsConfigFailed(e.to_string()))?;

    rustls_config.alpn_protocols = vec![
        b"bennett-p2p".to_vec(),
    ];

    Ok(quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(rustls_config)
            .map_err(|e| DtlsError::TlsConfigFailed(e.to_string()))?
    )))
}

/// Skip server certificate verification (P2P — we trust the share token for auth)
#[derive(Debug)]
struct SkipServerVerification;

impl rustls::client::danger::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::Verified, rustls::Error> {
        Ok(rustls::client::danger::Verified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
        ]
    }
}

use std::time::Duration;

/// DTLS/QUIC errors
#[derive(Debug)]
pub enum DtlsError {
    CertGenFailed(String),
    TlsConfigFailed(String),
}

impl std::fmt::Display for DtlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DtlsError::CertGenFailed(s) => write!(f, "Certificate generation failed: {}", s),
            DtlsError::TlsConfigFailed(s) => write!(f, "TLS config failed: {}", s),
        }
    }
}

impl std::error::Error for DtlsError {}
