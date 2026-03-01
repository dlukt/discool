-- Direct message persistence for Story 6.9.
-- Compatible with SQLite + PostgreSQL.

CREATE TABLE IF NOT EXISTS dm_messages (
    id TEXT PRIMARY KEY,
    dm_channel_id TEXT NOT NULL REFERENCES dm_channels(id) ON DELETE CASCADE,
    author_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    is_system INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT ck_dm_messages_is_system_boolean CHECK (is_system IN (0, 1))
);

CREATE INDEX IF NOT EXISTS idx_dm_messages_dm_channel_id_created_at_id
    ON dm_messages(dm_channel_id, created_at, id);

CREATE INDEX IF NOT EXISTS idx_dm_messages_author_user_id_created_at
    ON dm_messages(author_user_id, created_at);
