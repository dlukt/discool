-- Role persistence foundation for Story 5.1.
-- Compatible with SQLite + PostgreSQL.

CREATE TABLE IF NOT EXISTS roles (
    id TEXT PRIMARY KEY,
    guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    color TEXT NOT NULL,
    position INTEGER NOT NULL,
    permissions_bitflag INTEGER NOT NULL,
    is_default INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT ck_roles_position_non_negative CHECK (position >= 0),
    CONSTRAINT ck_roles_is_default CHECK (is_default IN (0, 1))
);

CREATE INDEX IF NOT EXISTS idx_roles_guild_id ON roles(guild_id);
CREATE INDEX IF NOT EXISTS idx_roles_guild_id_position ON roles(guild_id, is_default, position);
CREATE INDEX IF NOT EXISTS idx_roles_guild_id_name ON roles(guild_id, name);

CREATE TABLE IF NOT EXISTS role_assignments (
    guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    assigned_at TEXT NOT NULL,
    CONSTRAINT pk_role_assignments PRIMARY KEY (guild_id, user_id, role_id)
);

CREATE INDEX IF NOT EXISTS idx_role_assignments_role_id ON role_assignments(role_id);
CREATE INDEX IF NOT EXISTS idx_role_assignments_guild_user ON role_assignments(guild_id, user_id);

-- Backfill one @everyone role per existing guild so legacy guilds satisfy Story 5.1 defaults.
INSERT INTO roles (
    id,
    guild_id,
    name,
    color,
    position,
    permissions_bitflag,
    is_default,
    created_at,
    updated_at
)
SELECT
    'role-everyone-' || g.id,
    g.id,
    '@everyone',
    '#99aab5',
    2147483647,
    0,
    1,
    g.created_at,
    g.updated_at
FROM guilds g
WHERE NOT EXISTS (
    SELECT 1
    FROM roles r
    WHERE r.guild_id = g.id
      AND r.is_default = 1
);
