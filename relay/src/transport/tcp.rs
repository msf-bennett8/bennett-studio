//! TCP Transport — direct connection to local engine
//! This is the active, production-ready transport

use super::{ProtocolType, Transport};
use async_trait::async_trait;
use std::io;
use tokio::net::TcpStream;
use tracing::{debug, error, info};

/// TCP transport connecting to local engine
pub struct TcpTransport {
    engine_http: std::net::SocketAddr,
    engine_mysql: std::net::SocketAddr,
}

impl TcpTransport {
    pub fn new(
        engine_http: std::net::SocketAddr,
        engine_mysql: std::net::SocketAddr,
    ) -> Self {
        Self {
            engine_http,
            engine_mysql,
        }
    }

    /// Get the target address for a protocol
    fn target(&self, protocol: ProtocolType) -> std::net::SocketAddr {
        match protocol {
            ProtocolType::ConnectRpc | ProtocolType::Grpc => self.engine_http,
            ProtocolType::MySqlWire => self.engine_mysql,
        }
    }
}

#[async_trait]
impl Transport for TcpTransport {
    fn name(&self) -> &'static str {
        "tcp"
    }

    async fn connect(
        &self,
        share_id: &str,
        protocol: ProtocolType,
    ) -> io::Result<TcpStream> {
        let target = self.target(protocol);
        debug!(
            share_id = %share_id,
            transport = "tcp",
            target = %target,
            protocol = ?protocol,
            "Connecting to engine"
        );

        match TcpStream::connect(target).await {
            Ok(stream) => {
                info!(
                    share_id = %share_id,
                    target = %target,
                    "Connected to engine via TCP"
                );
                Ok(stream)
            }
            Err(e) => {
                error!(
                    share_id = %share_id,
                    target = %target,
                    error = %e,
                    "Failed to connect to engine"
                );
                Err(e)
            }
        }
    }

    async fn health_check(&self) -> bool {
        // Try to connect to both engine endpoints
        let http_ok = TcpStream::connect(self.engine_http).await.is_ok();
        let mysql_ok = TcpStream::connect(self.engine_mysql).await.is_ok();

        if http_ok && mysql_ok {
            debug!("TCP transport health check: OK");
        } else {
            error!(
                http_ok = http_ok,
                mysql_ok = mysql_ok,
                "TCP transport health check: FAILED"
            );
        }

        http_ok && mysql_ok
    }
}
