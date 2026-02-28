-- Cursor pagination index for Story 6.3.
-- Keeps deterministic scans for ORDER BY (created_at, id).

CREATE INDEX IF NOT EXISTS idx_messages_channel_id_created_at_id
    ON messages(channel_id, created_at, id);
