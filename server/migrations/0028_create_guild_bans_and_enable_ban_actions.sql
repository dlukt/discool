-- Introduce persistent guild bans and allow moderation_actions to track ban events.
-- Compatible with SQLite + PostgreSQL by rebuilding moderation_actions with updated constraints.

CREATE TABLE guild_bans (
    id TEXT PRIMARY KEY,
    guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    target_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    actor_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reason TEXT NOT NULL,
    delete_messages_window_seconds BIGINT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    unbanned_by_user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    unbanned_at TEXT,
    CONSTRAINT ck_guild_bans_actor_target CHECK (actor_user_id <> target_user_id),
    CONSTRAINT ck_guild_bans_reason_non_empty CHECK (LENGTH(TRIM(reason)) > 0),
    CONSTRAINT ck_guild_bans_delete_window_positive CHECK (
        delete_messages_window_seconds IS NULL OR delete_messages_window_seconds > 0
    ),
    CONSTRAINT ck_guild_bans_active_flag CHECK (is_active IN (0, 1)),
    CONSTRAINT ck_guild_bans_unban_pair CHECK (
        (unbanned_at IS NULL AND unbanned_by_user_id IS NULL)
        OR (unbanned_at IS NOT NULL AND unbanned_by_user_id IS NOT NULL)
    )
);

CREATE UNIQUE INDEX idx_guild_bans_unique_active_target
    ON guild_bans(guild_id, target_user_id)
    WHERE is_active = 1;

CREATE INDEX idx_guild_bans_active_lookup
    ON guild_bans(guild_id, target_user_id, is_active);

CREATE INDEX idx_guild_bans_guild_created_at
    ON guild_bans(guild_id, created_at);

CREATE TABLE moderation_actions_new (
    id TEXT PRIMARY KEY,
    action_type TEXT NOT NULL,
    guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    actor_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reason TEXT NOT NULL,
    duration_seconds BIGINT,
    expires_at TEXT,
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT ck_moderation_actions_type CHECK (action_type IN ('mute', 'kick', 'ban')),
    CONSTRAINT ck_moderation_actions_actor_target CHECK (actor_user_id <> target_user_id),
    CONSTRAINT ck_moderation_actions_duration_positive CHECK (
        duration_seconds IS NULL OR duration_seconds > 0
    ),
    CONSTRAINT ck_moderation_actions_duration_expiry_pair CHECK (
        (duration_seconds IS NULL AND expires_at IS NULL)
        OR (duration_seconds IS NOT NULL AND expires_at IS NOT NULL)
    ),
    CONSTRAINT ck_moderation_actions_active_flag CHECK (is_active IN (0, 1))
);

INSERT INTO moderation_actions_new (
    id,
    action_type,
    guild_id,
    actor_user_id,
    target_user_id,
    reason,
    duration_seconds,
    expires_at,
    is_active,
    created_at,
    updated_at
)
SELECT
    id,
    action_type,
    guild_id,
    actor_user_id,
    target_user_id,
    reason,
    duration_seconds,
    expires_at,
    is_active,
    created_at,
    updated_at
FROM moderation_actions;

DROP TABLE moderation_actions;
ALTER TABLE moderation_actions_new RENAME TO moderation_actions;

CREATE INDEX IF NOT EXISTS idx_moderation_actions_active_mute_by_target
    ON moderation_actions(guild_id, target_user_id, action_type, is_active, created_at);

CREATE INDEX IF NOT EXISTS idx_moderation_actions_expiration_sweep
    ON moderation_actions(action_type, is_active, expires_at);
