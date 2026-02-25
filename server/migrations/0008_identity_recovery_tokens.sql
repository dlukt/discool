-- Identity recovery token lifecycle for Story 2.7.
-- Compatible with SQLite and PostgreSQL.

CREATE TABLE IF NOT EXISTS identity_recovery_tokens (
    token_hash TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    requester_ip TEXT NOT NULL,
    used_by_ip TEXT,
    expires_at TEXT NOT NULL,
    used_at TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_identity_recovery_tokens_user_id
    ON identity_recovery_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_identity_recovery_tokens_requester_ip_created_at
    ON identity_recovery_tokens(requester_ip, created_at);
CREATE INDEX IF NOT EXISTS idx_identity_recovery_tokens_expires_at
    ON identity_recovery_tokens(expires_at);
CREATE INDEX IF NOT EXISTS idx_identity_recovery_tokens_used_at
    ON identity_recovery_tokens(used_at);

CREATE TABLE IF NOT EXISTS identity_recovery_start_attempts (
    requester_ip TEXT NOT NULL,
    attempted_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_identity_recovery_start_attempts_ip_attempted_at
    ON identity_recovery_start_attempts(requester_ip, attempted_at);

CREATE TABLE IF NOT EXISTS identity_recovery_redeem_attempts (
    requester_ip TEXT NOT NULL,
    attempted_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_identity_recovery_redeem_attempts_ip_attempted_at
    ON identity_recovery_redeem_attempts(requester_ip, attempted_at);
