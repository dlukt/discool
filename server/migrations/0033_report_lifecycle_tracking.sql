-- Track report lifecycle metadata for moderation queue actions and enforce lifecycle consistency.
-- Compatible with SQLite + PostgreSQL by rebuilding reports with explicit constraints.

CREATE TABLE reports_new (
    id TEXT PRIMARY KEY,
    guild_id TEXT NOT NULL REFERENCES guilds(id) ON DELETE CASCADE,
    reporter_user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type TEXT NOT NULL,
    target_message_id TEXT REFERENCES messages(id) ON DELETE CASCADE,
    target_user_id TEXT REFERENCES users(id) ON DELETE CASCADE,
    reason TEXT NOT NULL,
    category TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    reviewed_at TEXT,
    reviewed_by_user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    actioned_at TEXT,
    actioned_by_user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    dismissed_at TEXT,
    dismissed_by_user_id TEXT REFERENCES users(id) ON DELETE SET NULL,
    dismissal_reason TEXT,
    moderation_action_id TEXT REFERENCES moderation_actions(id) ON DELETE SET NULL,
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
    ),
    CONSTRAINT ck_reports_dismissal_reason_max_length CHECK (
        dismissal_reason IS NULL OR LENGTH(dismissal_reason) <= 500
    ),
    CONSTRAINT ck_reports_lifecycle_consistency CHECK (
        (
            status = 'pending'
            AND reviewed_at IS NULL
            AND reviewed_by_user_id IS NULL
            AND actioned_at IS NULL
            AND actioned_by_user_id IS NULL
            AND dismissed_at IS NULL
            AND dismissed_by_user_id IS NULL
            AND dismissal_reason IS NULL
            AND moderation_action_id IS NULL
        )
        OR (
            status = 'reviewed'
            AND reviewed_at IS NOT NULL
            AND reviewed_by_user_id IS NOT NULL
            AND actioned_at IS NULL
            AND actioned_by_user_id IS NULL
            AND dismissed_at IS NULL
            AND dismissed_by_user_id IS NULL
            AND dismissal_reason IS NULL
            AND moderation_action_id IS NULL
        )
        OR (
            status = 'actioned'
            AND reviewed_at IS NOT NULL
            AND reviewed_by_user_id IS NOT NULL
            AND actioned_at IS NOT NULL
            AND actioned_by_user_id IS NOT NULL
            AND dismissed_at IS NULL
            AND dismissed_by_user_id IS NULL
            AND dismissal_reason IS NULL
        )
        OR (
            status = 'dismissed'
            AND reviewed_at IS NOT NULL
            AND reviewed_by_user_id IS NOT NULL
            AND actioned_at IS NULL
            AND actioned_by_user_id IS NULL
            AND dismissed_at IS NOT NULL
            AND dismissed_by_user_id IS NOT NULL
            AND moderation_action_id IS NULL
        )
    )
);

INSERT INTO reports_new (
    id,
    guild_id,
    reporter_user_id,
    target_type,
    target_message_id,
    target_user_id,
    reason,
    category,
    status,
    reviewed_at,
    reviewed_by_user_id,
    actioned_at,
    actioned_by_user_id,
    dismissed_at,
    dismissed_by_user_id,
    dismissal_reason,
    moderation_action_id,
    created_at,
    updated_at
)
SELECT
    id,
    guild_id,
    reporter_user_id,
    target_type,
    target_message_id,
    target_user_id,
    reason,
    category,
    status,
    CASE
        WHEN status IN ('reviewed', 'actioned', 'dismissed') THEN updated_at
        ELSE NULL
    END AS reviewed_at,
    CASE
        WHEN status IN ('reviewed', 'actioned', 'dismissed') THEN reporter_user_id
        ELSE NULL
    END AS reviewed_by_user_id,
    CASE
        WHEN status = 'actioned' THEN updated_at
        ELSE NULL
    END AS actioned_at,
    CASE
        WHEN status = 'actioned' THEN reporter_user_id
        ELSE NULL
    END AS actioned_by_user_id,
    CASE
        WHEN status = 'dismissed' THEN updated_at
        ELSE NULL
    END AS dismissed_at,
    CASE
        WHEN status = 'dismissed' THEN reporter_user_id
        ELSE NULL
    END AS dismissed_by_user_id,
    NULL AS dismissal_reason,
    NULL AS moderation_action_id,
    created_at,
    updated_at
FROM reports;

DROP TABLE reports;
ALTER TABLE reports_new RENAME TO reports;

CREATE UNIQUE INDEX IF NOT EXISTS idx_reports_unique_message_target
    ON reports(guild_id, reporter_user_id, target_type, target_message_id)
    WHERE target_type = 'message';

CREATE UNIQUE INDEX IF NOT EXISTS idx_reports_unique_user_target
    ON reports(guild_id, reporter_user_id, target_type, target_user_id)
    WHERE target_type = 'user';

CREATE INDEX IF NOT EXISTS idx_reports_queue_guild_status_created
    ON reports(guild_id, status, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_reports_queue_pending_created
    ON reports(guild_id, created_at DESC, id DESC)
    WHERE status = 'pending';

CREATE INDEX IF NOT EXISTS idx_reports_queue_guild_created
    ON reports(guild_id, created_at DESC, id DESC);
