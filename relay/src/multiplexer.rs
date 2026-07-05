//! TCP multiplexer — bidirectional byte forwarding
//! Bridges client TLS stream ↔ engine TCP stream

use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tracing::{debug, error, info, warn};

/// Forward bytes between two streams until one closes
pub async fn proxy_bidirectional<A, B>(
    mut client: A,
    mut engine: B,
    share_id: String,
    protocol: &'static str,
) -> std::io::Result<()>
where
    A: AsyncRead + AsyncWrite + Unpin,
    B: AsyncRead + AsyncWrite + Unpin,
{
    use tokio::io::copy;

    info!(
        share_id = %share_id,
        protocol = protocol,
        "Starting bidirectional proxy"
    );

    let (mut client_read, mut client_write) = tokio::io::split(client);
    let (mut engine_read, mut engine_write) = tokio::io::split(engine);

    let client_to_engine = async {
        match copy(&mut client_read, &mut engine_write).await {
            Ok(n) => {
                debug!(
                    share_id = %share_id,
                    bytes = n,
                    "Client → Engine stream closed"
                );
            }
            Err(e) => {
                warn!(
                    share_id = %share_id,
                    error = %e,
                    "Client → Engine error"
                );
            }
        }
    };

    let engine_to_client = async {
        match copy(&mut engine_read, &mut client_write).await {
            Ok(n) => {
                debug!(
                    share_id = %share_id,
                    bytes = n,
                    "Engine → Client stream closed"
                );
            }
            Err(e) => {
                warn!(
                    share_id = %share_id,
                    error = %e,
                    "Engine → Client error"
                );
            }
        }
    };

    tokio::select! {
        _ = client_to_engine => {},
        _ = engine_to_client => {},
    }

    info!(
        share_id = %share_id,
        "Bidirectional proxy closed"
    );

    Ok(())
}

/// Extract share_id from HTTP request path
/// Format: /db/SHARE_ID or /api/shares/SHARE_ID/...
pub fn extract_share_id_from_http_path(path: &str) -> Option<String> {
    // Path patterns:
    // /db/AG5BECGUT9?t=...
    // /api/shares/AG5BECGUT9/validate
    // /bennett.v1.QueryService/ExecuteQuery (share in body, not path)

    if let Some(rest) = path.strip_prefix("/db/") {
        // Extract share code before query params
        let code = rest.split('?').next().unwrap_or(rest);
        return Some(code.to_string());
    }

    if let Some(rest) = path.strip_prefix("/api/shares/") {
        let parts: Vec<&str> = rest.split('/').collect();
        if !parts.is_empty() && !parts[0].is_empty() {
            return Some(parts[0].to_string());
        }
    }

    None
}

/// Extract share_id from MySQL wire protocol handshake
/// The username field contains the share code
pub fn extract_share_id_from_mysql_username(username: &str) -> String {
    // MySQL clients send username as: "bennett_SHARECODE" or just "SHARECODE"
    if let Some(code) = username.strip_prefix("bennett_") {
        return code.to_string();
    }
    username.to_string()
}
