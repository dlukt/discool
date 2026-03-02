-- Moderation action persistence for Epic 8 Story 8.1 (mute lifecycle).
-- Compatible with SQLite + PostgreSQL.

CREATE TABLE IF NOT EXISTS moderation_actions (
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
    CONSTRAINT ck_moderation_actions_type CHECK (action_type = 'mute'),
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

CREATE INDEX IF NOT EXISTS idx_moderation_actions_active_mute_by_target
    ON moderation_actions(guild_id, target_user_id, action_type, is_active, created_at);

CREATE INDEX IF NOT EXISTS idx_moderation_actions_expiration_sweep
    ON moderation_actions(action_type, is_active, expires_at);
