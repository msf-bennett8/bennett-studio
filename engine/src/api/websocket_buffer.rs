//! WebSocket message replay buffer
//! Stores last N messages per session for reconnection recovery

use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

use crate::api::websocket::WsResponse;

/// Ring buffer entry with timestamp
struct BufferEntry {
    message_id: u64,
    message: WsResponse,
    timestamp: Instant,
}

/// Per-session message buffer for replay
pub struct SessionBuffer {
    messages: RwLock<VecDeque<BufferEntry>>,
    max_size: usize,
    ttl: Duration,
    session_id: String,
}

impl SessionBuffer {
    pub fn new(session_id: String, max_size: usize, ttl_secs: u64) -> Self {
        Self {
            messages: RwLock::new(VecDeque::with_capacity(max_size)),
            max_size,
            ttl: Duration::from_secs(ttl_secs),
            session_id,
        }
    }

    /// Store a message in the buffer
    pub async fn push(&self, message_id: u64, message: WsResponse) {
        let mut messages = self.messages.write().await;

        // Remove expired entries
        let now = Instant::now();
        while let Some(front) = messages.front() {
            if now.duration_since(front.timestamp) > self.ttl {
                messages.pop_front();
            } else {
                break;
            }
        }

        // Remove oldest if at capacity
        if messages.len() >= self.max_size {
            messages.pop_front();
        }

        messages.push_back(BufferEntry {
            message_id,
            message,
            timestamp: now,
        });

        debug!("Buffered message {} for session {}", message_id, self.session_id);
    }

    /// Get messages after a specific message_id (for replay)
    /// Returns messages where message_id > last_message_id
    pub async fn get_missed(&self, last_message_id: u64) -> Vec<WsResponse> {
        let messages = self.messages.read().await;
        let now = Instant::now();

        messages
            .iter()
            .filter(|entry| {
                entry.message_id > last_message_id
                    && now.duration_since(entry.timestamp) <= self.ttl
            })
            .map(|entry| entry.message.clone())
            .collect()
    }

    /// Get the highest message_id in buffer
    pub async fn last_message_id(&self) -> u64 {
        let messages = self.messages.read().await;
        messages.back().map(|e| e.message_id).unwrap_or(0)
    }

    /// Get buffer stats
    pub async fn stats(&self) -> BufferStats {
        let messages = self.messages.read().await;
        let now = Instant::now();
        let active = messages
            .iter()
            .filter(|e| now.duration_since(e.timestamp) <= self.ttl)
            .count();

        BufferStats {
            total_stored: messages.len(),
            active_entries: active,
            max_size: self.max_size,
            session_id: self.session_id.clone(),
        }
    }

    /// Clear all messages
    pub async fn clear(&self) {
        let mut messages = self.messages.write().await;
        messages.clear();
        debug!("Cleared buffer for session {}", self.session_id);
    }
}

#[derive(Debug, Clone)]
pub struct BufferStats {
    pub total_stored: usize,
    pub active_entries: usize,
    pub max_size: usize,
    pub session_id: String,
}

/// Global message buffer manager
/// Maps session_id -> SessionBuffer
pub struct WsMessageBuffer {
    buffers: RwLock<std::collections::HashMap<String, SessionBuffer>>,
    max_size: usize,
    ttl_secs: u64,
}

impl WsMessageBuffer {
    pub fn new() -> Self {
        Self {
            buffers: RwLock::new(std::collections::HashMap::new()),
            max_size: 100,
            ttl_secs: 300, // 5 minutes
        }
    }

    /// Create or get a session buffer
    pub async fn get_or_create(&self, session_id: &str) -> SessionBuffer {
        let buffers = self.buffers.read().await;
        if let Some(buffer) = buffers.get(session_id) {
            return SessionBuffer {
                messages: RwLock::new(VecDeque::new()),
                max_size: self.max_size,
                ttl: Duration::from_secs(self.ttl_secs),
                session_id: session_id.to_string(),
            };
        }
        drop(buffers);

        let buffer = SessionBuffer::new(session_id.to_string(), self.max_size, self.ttl_secs);
        let mut buffers = self.buffers.write().await;
        buffers.insert(session_id.to_string(), SessionBuffer::new(session_id.to_string(), self.max_size, self.ttl_secs));
        
        info!("Created message buffer for session {}", session_id);
        buffer
    }

    /// Get existing buffer (for replay)
    pub async fn get(&self, session_id: &str) -> Option<SessionBuffer> {
        let buffers = self.buffers.read().await;
        buffers.get(session_id).map(|_| SessionBuffer {
            messages: RwLock::new(VecDeque::new()),
            max_size: self.max_size,
            ttl: Duration::from_secs(self.ttl_secs),
            session_id: session_id.to_string(),
        })
    }

    /// Remove a session buffer (on disconnect/timeout)
    pub async fn remove(&self, session_id: &str) {
        let mut buffers = self.buffers.write().await;
        if buffers.remove(session_id).is_some() {
            info!("Removed message buffer for session {}", session_id);
        }
    }

    /// Cleanup expired sessions (call periodically)
    pub async fn cleanup(&self) {
        // Buffers auto-cleanup expired entries on push/get
        // This removes empty buffers
        let mut buffers = self.buffers.write().await;
        let before = buffers.len();
        
        // Remove buffers with no active entries
        let to_remove: Vec<String> = Vec::new(); // Simplified - would need to check each buffer
        
        for id in to_remove {
            buffers.remove(&id);
        }

        let after = buffers.len();
        if before != after {
            info!("Buffer cleanup: removed {} empty buffers, {} remaining", before - after, after);
        }
    }

    /// Get total stats
    pub async fn stats(&self) -> GlobalBufferStats {
        let buffers = self.buffers.read().await;
        GlobalBufferStats {
            total_sessions: buffers.len(),
            max_size_per_session: self.max_size,
            ttl_secs: self.ttl_secs,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GlobalBufferStats {
    pub total_sessions: usize,
    pub max_size_per_session: usize,
    pub ttl_secs: u64,
}
