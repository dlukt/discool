-- Message persistence for Story 6.2.
-- Compatible with SQLite + PostgreSQL.

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    channel_id TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    author_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    is_system INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT ck_messages_is_system_boolean CHECK (is_system IN (0, 1))
);

CREATE INDEX IF NOT EXISTS idx_messages_channel_id_created_at
    ON messages(channel_id, created_at);
