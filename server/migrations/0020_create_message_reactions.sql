-- Emoji reactions persistence for Story 6.5.
-- Compatible with SQLite + PostgreSQL.

CREATE TABLE IF NOT EXISTS message_reactions (
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    emoji TEXT NOT NULL,
    created_at TEXT NOT NULL,
    CONSTRAINT pk_message_reactions PRIMARY KEY (message_id, user_id, emoji),
    CONSTRAINT ck_message_reactions_emoji_non_empty CHECK (LENGTH(TRIM(emoji)) > 0)
);

CREATE INDEX IF NOT EXISTS idx_message_reactions_message_id_emoji_created_at
    ON message_reactions(message_id, emoji, created_at);
