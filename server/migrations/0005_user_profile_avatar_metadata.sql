-- User profile extension: display_name + avatar file metadata.
-- Compatible with both PostgreSQL and SQLite.

ALTER TABLE users ADD COLUMN display_name TEXT;
ALTER TABLE users ADD COLUMN avatar_storage_key TEXT;
ALTER TABLE users ADD COLUMN avatar_mime_type TEXT;
ALTER TABLE users ADD COLUMN avatar_size_bytes INTEGER;
ALTER TABLE users ADD COLUMN avatar_updated_at TEXT;

