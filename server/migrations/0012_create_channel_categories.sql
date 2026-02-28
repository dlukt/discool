CREATE TABLE IF NOT EXISTS channel_categories (
    id TEXT PRIMARY KEY,
    guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    slug TEXT NOT NULL,
    name TEXT NOT NULL,
    position INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT uq_channel_categories_guild_slug UNIQUE (guild_id, slug),
    CONSTRAINT ck_channel_categories_position_non_negative CHECK (position >= 0)
);

CREATE INDEX IF NOT EXISTS idx_channel_categories_guild_id ON channel_categories(guild_id);
CREATE INDEX IF NOT EXISTS idx_channel_categories_guild_id_position ON channel_categories(guild_id, position);

ALTER TABLE channels
ADD COLUMN category_id TEXT REFERENCES channel_categories(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_channels_guild_id_category_id_position ON channels(guild_id, category_id, position);

CREATE TABLE IF NOT EXISTS channel_category_collapses (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    category_id TEXT NOT NULL REFERENCES channel_categories(id) ON DELETE CASCADE,
    collapsed INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT pk_channel_category_collapses PRIMARY KEY (user_id, guild_id, category_id),
    CONSTRAINT ck_channel_category_collapses_collapsed CHECK (collapsed IN (0, 1))
);

CREATE INDEX IF NOT EXISTS idx_channel_category_collapses_user_guild ON channel_category_collapses(user_id, guild_id);
