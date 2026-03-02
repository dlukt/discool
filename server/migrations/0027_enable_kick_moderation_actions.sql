-- Enable kick moderation actions alongside mute in moderation_actions.
-- Compatible with SQLite + PostgreSQL by rebuilding the table with updated check constraints.

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
    CONSTRAINT ck_moderation_actions_type CHECK (action_type IN ('mute', 'kick')),
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
