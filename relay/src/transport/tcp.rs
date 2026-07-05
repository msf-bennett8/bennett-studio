//! Pooled TCP Transport with Linux splice() zero-copy
//!
//! Architecture:
//! - Maintains connection pools per protocol (HTTP, MySQL)
//! - Reuses connections (eliminates SYN/ACK overhead)
//! - Uses splice() on Linux for kernel-bypass forwarding
//! - Falls back to tokio::io::copy_bidirectional on non-Linux

use super::{ProtocolType, PooledConnection, Transport};
use std::collections::VecDeque;
use std::future::Future;
use std::io;
use std::os::fd::AsRawFd;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::net::TcpStream;
use tokio::time::{interval, Duration};
use tracing::{debug, warn};

/// Connection pool for a single protocol
struct ProtocolPool {
    available: Mutex<VecDeque<PooledConnection>>,
    in_use: AtomicUsize,
    max_size: usize,
    target: std::net::SocketAddr,
    protocol: ProtocolType,
}

impl ProtocolPool {
    fn new(target: std::net::SocketAddr, protocol: ProtocolType, max_size: usize) -> Self {
        Self {
            available: Mutex::new(VecDeque::with_capacity(max_size)),
            in_use: AtomicUsize::new(0),
            max_size,
            target,
            protocol,
        }
    }

    /// Get connection from pool or create new
    async fn acquire(&self) -> io::Result<PooledConnection> {
        // Try to get from pool first
        {
            let mut available = self.available.lock().unwrap();
            while let Some(mut conn) = available.pop_front() {
                // Check if connection is still alive
                if !conn.is_stale(300) && is_connection_alive(&conn.stream).await {
                    self.in_use.fetch_add(1, Ordering::Relaxed);
                    debug!(protocol = ?self.protocol, "Reused pooled connection");
                    return Ok(conn);
                }
                // Stale or dead, drop it
                debug!(protocol = ?self.protocol, "Dropped stale/dead pooled connection");
            }
        }

        // Create new connection
        debug!(target = %self.target, protocol = ?self.protocol, "Creating new connection");
        let stream = TcpStream::connect(self.target).await?;
        self.in_use.fetch_add(1, Ordering::Relaxed);

        Ok(PooledConnection {
            stream,
            protocol: self.protocol,
            created_at: Instant::now(),
        })
    }

    /// Return connection to pool
    fn release(&self, conn: PooledConnection) {
        self.in_use.fetch_sub(1, Ordering::Relaxed);

        let mut available = self.available.lock().unwrap();
        if available.len() < self.max_size && is_connection_alive_sync(&conn.stream) {
            available.push_back(conn);
            debug!(protocol = ?self.protocol, pool_size = available.len(), "Returned connection to pool");
        } else {
            debug!(protocol = ?self.protocol, "Dropped connection (pool full or dead)");
        }
    }

    fn in_use(&self) -> usize {
        self.in_use.load(Ordering::Relaxed)
    }

    fn available(&self) -> usize {
        self.available.lock().unwrap().len()
    }
}

/// Pooled TCP transport with per-protocol connection pools
pub struct PooledTcpTransport {
    http_pool: Arc<ProtocolPool>,
    mysql_pool: Arc<ProtocolPool>,
}

impl PooledTcpTransport {
    pub fn new(
        engine_http: std::net::SocketAddr,
        engine_mysql: std::net::SocketAddr,
        pool_size: usize,
    ) -> Self {
        let http_pool = Arc::new(ProtocolPool::new(engine_http, ProtocolType::ConnectRpc, pool_size));
        let mysql_pool = Arc::new(ProtocolPool::new(engine_mysql, ProtocolType::MySqlWire, pool_size));

        // Start background janitor
        let http_clone = http_pool.clone();
        let mysql_clone = mysql_pool.clone();
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(60));
            loop {
                ticker.tick().await;
                janitor_cleanup(&http_clone);
                janitor_cleanup(&mysql_clone);
            }
        });

        Self {
            http_pool,
            mysql_pool,
        }
    }

    fn pool_for(&self, protocol: ProtocolType) -> &Arc<ProtocolPool> {
        match protocol {
            ProtocolType::ConnectRpc | ProtocolType::Grpc => &self.http_pool,
            ProtocolType::MySqlWire => &self.mysql_pool,
        }
    }
}

impl Transport for PooledTcpTransport {
    fn name(&self) -> &'static str {
        "pooled-tcp"
    }

    fn acquire(
        &self,
        protocol: ProtocolType,
    ) -> Pin<Box<dyn Future<Output = io::Result<PooledConnection>> + Send + '_>> {
        let pool = self.pool_for(protocol).clone();
        Box::pin(async move { pool.acquire().await })
    }

    fn release(&self, conn: PooledConnection) {
        self.pool_for(conn.protocol).release(conn);
    }

    fn health_check(&self) -> Pin<Box<dyn Future<Output = bool> + Send + '_>> {
        let http_pool = self.http_pool.clone();
        let mysql_pool = self.mysql_pool.clone();
        Box::pin(async move {
            let http_ok = http_pool.acquire().await.is_ok();
            let mysql_ok = mysql_pool.acquire().await.is_ok();
            http_ok && mysql_ok
        })
    }
}

/// Cleanup stale connections from pool
fn janitor_cleanup(pool: &ProtocolPool) {
    let mut available = pool.available.lock().unwrap();
    let before = available.len();
    available.retain(|conn| !conn.is_stale(300) && is_connection_alive_sync(&conn.stream));
    let after = available.len();
    if before != after {
        debug!(protocol = ?pool.protocol, dropped = before - after, "Janitor cleaned stale connections");
    }
}

/// Check if TCP connection is still alive (async version)
async fn is_connection_alive(stream: &TcpStream) -> bool {
    // Try to peek 0 bytes — non-blocking liveness check
    stream.peek(&mut []).await.is_ok()
}

/// Check if TCP connection is still alive (sync version for janitor)
fn is_connection_alive_sync(stream: &TcpStream) -> bool {
    // Check if socket has error
    let fd = stream.as_raw_fd();
    let mut error = 0;
    let mut len = std::mem::size_of::<libc::c_int>() as libc::socklen_t;
    unsafe {
        libc::getsockopt(fd, libc::SOL_SOCKET, libc::SO_ERROR, &mut error as *mut _ as *mut libc::c_void, &mut len);
    }
    error == 0
}

// ============================================================================
// Linux splice() zero-copy forwarding
// ============================================================================

/// Forward between two TCP streams using splice() on Linux
/// Falls back to tokio::io::copy_bidirectional on other platforms
pub async fn forward_zero_copy(
    client: &mut TcpStream,
    engine: &mut TcpStream,
) -> io::Result<(u64, u64)> {
    #[cfg(target_os = "linux")]
    {
        match splice_forward(client, engine).await {
            Ok(result) => return Ok(result),
            Err(e) => {
                warn!("splice() failed ({}), falling back to userspace copy", e);
            }
        }
    }
    
    // Fallback: tokio::io::copy_bidirectional
    tokio::io::copy_bidirectional(client, engine).await
}

#[cfg(target_os = "linux")]
async fn splice_forward(
    client: &mut TcpStream,
    engine: &mut TcpStream,
) -> io::Result<(u64, u64)> {
    use nix::fcntl::{splice, SpliceFFlags};
    
    let client_fd = client.as_raw_fd();
    let engine_fd = engine.as_raw_fd();
    
    // Create a pipe for each direction
    let (pipe_rd1, pipe_wr1) = nix::unistd::pipe()?;
    let (pipe_rd2, pipe_wr2) = nix::unistd::pipe()?;
    
    let mut total_client_to_engine = 0u64;
    let mut total_engine_to_client = 0u64;
    
    // Use tokio's blocking task for splice (it's a blocking syscall)
    let result = tokio::task::spawn_blocking(move || {
        let mut c2e_done = false;
        let mut e2c_done = false;
        
        while !c2e_done || !e2c_done {
            if !c2e_done {
                match splice(client_fd, None, pipe_wr1, None, 65536, SpliceFFlags::SPLICE_F_NONBLOCK | SpliceFFlags::SPLICE_F_MOVE) {
                    Ok(0) => c2e_done = true,
                    Ok(n) => {
                        total_client_to_engine += n as u64;
                        // Move from pipe to engine
                        let _ = splice(pipe_rd1, None, engine_fd, None, n, SpliceFFlags::SPLICE_F_NONBLOCK);
                    }
                    Err(nix::errno::Errno::EAGAIN) => {}
                    Err(e) => return Err(io::Error::from(e)),
                }
            }
            
            if !e2c_done {
                match splice(engine_fd, None, pipe_wr2, None, 65536, SpliceFFlags::SpliceFFlags::SPLICE_F_NONBLOCK | SpliceFFlags::SPLICE_F_MOVE) {
                    Ok(0) => e2c_done = true,
                    Ok(n) => {
                        total_engine_to_client += n as u64;
                        let _ = splice(pipe_rd2, None, client_fd, None, n, SpliceFFlags::SPLICE_F_NONBLOCK);
                    }
                    Err(nix::errno::Errno::EAGAIN) => {}
                    Err(e) => return Err(io::Error::from(e)),
                }
            }
        }
        
        Ok((total_client_to_engine, total_engine_to_client))
    }).await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))??;
    
    Ok(result)
}
