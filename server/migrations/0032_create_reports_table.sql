-- Add user-generated content reports for moderation intake.
-- Compatible with SQLite + PostgreSQL.

CREATE TABLE IF NOT EXISTS reports (
    id TEXT PRIMARY KEY,
    guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    reporter_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type TEXT NOT NULL,
    target_message_id TEXT REFERENCES messages(id) ON DELETE CASCADE,
    target_user_id TEXT REFERENCES users(id) ON DELETE CASCADE,
    reason TEXT NOT NULL,
    category TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT ck_reports_target_type CHECK (target_type IN ('message', 'user')),
    CONSTRAINT ck_reports_target_pair CHECK (
        (target_type = 'message' AND target_message_id IS NOT NULL AND target_user_id IS NULL)
        OR (target_type = 'user' AND target_user_id IS NOT NULL AND target_message_id IS NULL)
    ),
    CONSTRAINT ck_reports_reason_non_empty CHECK (LENGTH(TRIM(reason)) > 0),
    CONSTRAINT ck_reports_reason_max_length CHECK (LENGTH(reason) <= 500),
    CONSTRAINT ck_reports_category CHECK (
        category IS NULL OR category IN ('spam', 'harassment', 'rule_violation', 'other')
    ),
    CONSTRAINT ck_reports_status CHECK (
        status IN ('pending', 'reviewed', 'actioned', 'dismissed')
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_reports_unique_message_target
    ON reports(guild_id, reporter_user_id, target_type, target_message_id)
    WHERE target_type = 'message';

CREATE UNIQUE INDEX IF NOT EXISTS idx_reports_unique_user_target
    ON reports(guild_id, reporter_user_id, target_type, target_user_id)
    WHERE target_type = 'user';

CREATE INDEX IF NOT EXISTS idx_reports_queue_guild_status_created
    ON reports(guild_id, status, created_at, id);
