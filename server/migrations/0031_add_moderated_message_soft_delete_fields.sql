-- Add moderated message soft-delete metadata and history-focused visibility indexes.
-- Compatible with SQLite + PostgreSQL.

ALTER TABLE messages ADD COLUMN deleted_at TEXT;
ALTER TABLE messages ADD COLUMN deleted_by_user_id TEXT REFERENCES users(id) ON DELETE SET NULL;
ALTER TABLE messages ADD COLUMN deleted_reason TEXT;
ALTER TABLE messages ADD COLUMN deleted_moderation_action_id TEXT REFERENCES moderation_actions(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_messages_channel_visible_created_at_id
    ON messages(channel_id, deleted_at, created_at DESC, id DESC);

CREATE INDEX IF NOT EXISTS idx_messages_guild_author_visible_created_at
    ON messages(guild_id, author_user_id, deleted_at, created_at);
