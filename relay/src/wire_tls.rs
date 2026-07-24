//! Single self-signed TLS certificate for the public wire-protocol
//! (MySQL/Postgres) listener. Unlike the engine's per-share CertManager,
//! this is one cert for the whole listener — the same way a real MySQL
//! server has one TLS cert, not one per client.

use rcgen::{CertificateParams, KeyPair};
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;
use tracing::info;

pub fn build_wire_tls_acceptor(hostname: &str) -> anyhow::Result<TlsAcceptor> {
    let mut params = CertificateParams::new(vec![
        hostname.to_string(),
        "localhost".to_string(),
    ])?;
    params.subject_alt_names.push(rcgen::SanType::IpAddress(
        std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1))
    ));

    let key = KeyPair::generate()?;
    let cert = params.self_signed(&key)?;

    let cert_pem = cert.pem();
    let key_pem = key.serialize_pem();

    let cert_chain = rustls_pemfile::certs(&mut cert_pem.as_bytes())
        .collect::<Result<Vec<_>, _>>()?;
    let key_der = rustls_pemfile::private_key(&mut key_pem.as_bytes())?
        .ok_or_else(|| anyhow::anyhow!("No private key parsed"))?;

    let config = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(cert_chain, key_der)?;

    info!("Generated TLS certificate for wire-protocol listener ({})", hostname);
    Ok(TlsAcceptor::from(Arc::new(config)))
}
