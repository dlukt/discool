-- Channel-level role permission overrides for Story 5.5.
-- Compatible with SQLite + PostgreSQL.

CREATE TABLE IF NOT EXISTS channel_permission_overrides (
    channel_id TEXT NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    role_id TEXT NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    allow_bitflag INTEGER NOT NULL,
    deny_bitflag INTEGER NOT NULL,
    CONSTRAINT pk_channel_permission_overrides PRIMARY KEY (channel_id, role_id),
    CONSTRAINT ck_channel_permission_overrides_allow_non_negative CHECK (allow_bitflag >= 0),
    CONSTRAINT ck_channel_permission_overrides_deny_non_negative CHECK (deny_bitflag >= 0),
    CONSTRAINT ck_channel_permission_overrides_no_overlap CHECK ((allow_bitflag & deny_bitflag) = 0)
);

CREATE INDEX IF NOT EXISTS idx_channel_permission_overrides_channel_id
    ON channel_permission_overrides(channel_id);
CREATE INDEX IF NOT EXISTS idx_channel_permission_overrides_role_id
    ON channel_permission_overrides(role_id);

-- Backfill VIEW_CHANNEL (1 << 12) on existing roles to preserve legacy channel visibility.
UPDATE roles
SET permissions_bitflag = permissions_bitflag | 4096
WHERE (permissions_bitflag & 4096) = 0;
