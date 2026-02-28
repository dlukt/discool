-- Message attachments persistence for Story 6.6.
-- Compatible with SQLite + PostgreSQL.

CREATE TABLE IF NOT EXISTS message_attachments (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    storage_key TEXT NOT NULL,
    original_filename TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    CONSTRAINT ck_message_attachments_storage_key_non_empty CHECK (LENGTH(TRIM(storage_key)) > 0),
    CONSTRAINT ck_message_attachments_original_filename_non_empty CHECK (LENGTH(TRIM(original_filename)) > 0),
    CONSTRAINT ck_message_attachments_mime_type_non_empty CHECK (LENGTH(TRIM(mime_type)) > 0),
    CONSTRAINT ck_message_attachments_size_positive CHECK (size_bytes > 0)
);

CREATE INDEX IF NOT EXISTS idx_message_attachments_message_id_created_at
    ON message_attachments(message_id, created_at);
