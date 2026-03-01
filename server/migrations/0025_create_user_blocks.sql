-- User-level block interval persistence for Story 6.10.
-- Compatible with SQLite + PostgreSQL.

CREATE TABLE IF NOT EXISTS user_blocks (
    id TEXT PRIMARY KEY,
    owner_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_at TEXT NOT NULL,
    unblocked_at TEXT,
    blocked_user_display_name TEXT,
    blocked_user_username TEXT,
    blocked_user_avatar_color TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT ck_user_blocks_owner_not_self CHECK (owner_user_id <> blocked_user_id)
);

CREATE INDEX IF NOT EXISTS idx_user_blocks_owner_user_id
    ON user_blocks(owner_user_id);

CREATE INDEX IF NOT EXISTS idx_user_blocks_owner_blocked_user
    ON user_blocks(owner_user_id, blocked_user_id, blocked_at);
