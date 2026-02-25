-- Optional identity recovery email association + verification token lifecycle.
-- Compatible with SQLite and PostgreSQL.

CREATE TABLE IF NOT EXISTS user_recovery_email (
    user_id TEXT PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    normalized_email TEXT NOT NULL,
    email_masked TEXT NOT NULL,
    verified_at TEXT,
    encrypted_private_key TEXT,
    encryption_algorithm TEXT,
    encryption_version INTEGER,
    key_nonce TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_user_recovery_email_normalized_email
    ON user_recovery_email(normalized_email);
CREATE INDEX IF NOT EXISTS idx_user_recovery_email_verified_at
    ON user_recovery_email(verified_at);

CREATE TABLE IF NOT EXISTS email_verification_tokens (
    token_hash TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_email TEXT NOT NULL,
    email_masked TEXT NOT NULL,
    encrypted_private_key TEXT NOT NULL,
    encryption_algorithm TEXT NOT NULL,
    encryption_version INTEGER NOT NULL,
    key_nonce TEXT NOT NULL,
    requester_ip TEXT NOT NULL,
    used_by_ip TEXT,
    expires_at TEXT NOT NULL,
    used_at TEXT,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_email_verification_tokens_user_id
    ON email_verification_tokens(user_id);
CREATE INDEX IF NOT EXISTS idx_email_verification_tokens_requester_ip_created_at
    ON email_verification_tokens(requester_ip, created_at);
CREATE INDEX IF NOT EXISTS idx_email_verification_tokens_used_by_ip_used_at
    ON email_verification_tokens(used_by_ip, used_at);
CREATE INDEX IF NOT EXISTS idx_email_verification_tokens_expires_at
    ON email_verification_tokens(expires_at);
