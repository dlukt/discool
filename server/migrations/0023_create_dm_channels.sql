-- Direct message channel persistence for Story 6.9.
-- Compatible with SQLite + PostgreSQL.

CREATE TABLE IF NOT EXISTS dm_channels (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    user_low_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    user_high_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT ck_dm_channels_distinct_participants CHECK (user_low_id <> user_high_id),
    CONSTRAINT ck_dm_channels_canonical_pair CHECK (user_low_id < user_high_id),
    CONSTRAINT ck_dm_channels_slug_non_empty CHECK (LENGTH(TRIM(slug)) > 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_dm_channels_participant_pair
    ON dm_channels(user_low_id, user_high_id);

CREATE INDEX IF NOT EXISTS idx_dm_channels_user_low_updated_at
    ON dm_channels(user_low_id, updated_at);

CREATE INDEX IF NOT EXISTS idx_dm_channels_user_high_updated_at
    ON dm_channels(user_high_id, updated_at);
