-- Phase 1: Share system tables
-- Run manually or via sqlx migrate

CREATE TABLE IF NOT EXISTS shares (
    code TEXT PRIMARY KEY,
    db_id TEXT NOT NULL,
    host_id TEXT NOT NULL,
    token_jti TEXT NOT NULL UNIQUE,
    permission TEXT NOT NULL DEFAULT 'ro',
    tables TEXT NOT NULL DEFAULT '["*"]',
    cols TEXT,
    rls TEXT,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    revoked INTEGER NOT NULL DEFAULT 0,
    guest_count INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_shares_db_id ON shares(db_id);
CREATE INDEX IF NOT EXISTS idx_shares_expires ON shares(expires_at);
CREATE INDEX IF NOT EXISTS idx_shares_revoked ON shares(revoked);

CREATE TABLE IF NOT EXISTS guest_sessions (
    id TEXT PRIMARY KEY,
    share_code TEXT NOT NULL,
    ip_address TEXT,
    user_agent TEXT,
    connected_at TEXT NOT NULL,
    last_active TEXT NOT NULL,
    query_count INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (share_code) REFERENCES shares(code) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_guests_share ON guest_sessions(share_code);
CREATE INDEX IF NOT EXISTS idx_guests_last_active ON guest_sessions(last_active);

CREATE TABLE IF NOT EXISTS revoked_tokens (
    jti TEXT PRIMARY KEY,
    revoked_at TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT 'host_revoked'
);

CREATE INDEX IF NOT EXISTS idx_revoked_jti ON revoked_tokens(jti);
