-- Guild membership persistence for Story 4.6.
-- Compatible with SQLite + PostgreSQL.

CREATE TABLE IF NOT EXISTS guild_members (
    guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    joined_at TEXT NOT NULL,
    joined_via_invite_code TEXT,
    CONSTRAINT pk_guild_members PRIMARY KEY (guild_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_guild_members_user_id ON guild_members(user_id);
