-- User identity schema: DID:key + public key + profile basics.
-- Kept compatible with SQLite + PostgreSQL for the project's multi-db support.

CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY,
    did_key TEXT NOT NULL UNIQUE,
    public_key_multibase TEXT NOT NULL,
    username TEXT NOT NULL UNIQUE,
    avatar_color TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_users_did_key ON users(did_key);
CREATE INDEX IF NOT EXISTS idx_users_username ON users(username);

