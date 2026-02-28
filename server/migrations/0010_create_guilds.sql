-- Guild domain foundation for Story 4.2.
-- Compatible with SQLite + PostgreSQL.

CREATE TABLE IF NOT EXISTS guilds (
    id TEXT PRIMARY KEY,
    slug TEXT NOT NULL UNIQUE,
    name TEXT NOT NULL,
    description TEXT,
    owner_id TEXT NOT NULL REFERENCES users(id),
    default_channel_slug TEXT NOT NULL DEFAULT 'general',
    icon_storage_key TEXT,
    icon_mime_type TEXT,
    icon_size_bytes INTEGER,
    icon_updated_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_guilds_owner_id ON guilds(owner_id);
CREATE INDEX IF NOT EXISTS idx_guilds_slug ON guilds(slug);
