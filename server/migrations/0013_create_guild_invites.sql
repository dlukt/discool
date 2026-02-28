-- Guild invite persistence for Story 4.5.
-- Compatible with SQLite + PostgreSQL.

CREATE TABLE IF NOT EXISTS guild_invites (
    id TEXT PRIMARY KEY,
    guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    code TEXT NOT NULL UNIQUE,
    type TEXT NOT NULL,
    uses_remaining INTEGER NOT NULL,
    created_by TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    revoked INTEGER NOT NULL DEFAULT 0,
    CONSTRAINT ck_guild_invites_type CHECK (type IN ('reusable', 'single_use')),
    CONSTRAINT ck_guild_invites_uses_remaining_non_negative CHECK (uses_remaining >= 0),
    CONSTRAINT ck_guild_invites_revoked CHECK (revoked IN (0, 1))
);

CREATE INDEX IF NOT EXISTS idx_guild_invites_guild_id ON guild_invites(guild_id);
CREATE INDEX IF NOT EXISTS idx_guild_invites_revoked ON guild_invites(revoked);
CREATE INDEX IF NOT EXISTS idx_guild_invites_created_at ON guild_invites(created_at);
