-- Initial schema migration: verifies the migration system works.
-- Domain tables (users, guilds, channels, etc.) are created by their respective stories.

CREATE TABLE IF NOT EXISTS schema_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

INSERT INTO schema_metadata (key, value)
VALUES ('initialized_at', CURRENT_TIMESTAMP)
ON CONFLICT(key) DO NOTHING;

