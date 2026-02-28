-- Channel persistence foundation for Story 4.3.
-- Compatible with SQLite + PostgreSQL.

CREATE TABLE IF NOT EXISTS channels (
    id TEXT PRIMARY KEY,
    guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    channel_type TEXT NOT NULL,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT uq_channels_guild_slug UNIQUE (guild_id, slug),
    CONSTRAINT ck_channels_type CHECK (channel_type IN ('text', 'voice')),
    CONSTRAINT ck_channels_position_non_negative CHECK (position >= 0)
);

CREATE INDEX IF NOT EXISTS idx_channels_guild_id ON channels(guild_id);
CREATE INDEX IF NOT EXISTS idx_channels_guild_id_position ON channels(guild_id, position);

-- Backfill one default channel per existing guild so Story 4.2 data remains valid.
INSERT INTO channels (id, guild_id, slug, name, channel_type, position, created_at, updated_at)
SELECT
    'channel-' || g.id,
    g.id,
    g.default_channel_slug,
    g.default_channel_slug,
    'text',
    0,
    g.created_at,
    g.updated_at
FROM guilds g
WHERE NOT EXISTS (
    SELECT 1
    FROM channels c
    WHERE c.guild_id = g.id
);
