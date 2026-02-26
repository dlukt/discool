-- Persist discovered Discool instances for Story 3.2.
-- Compatible with SQLite and PostgreSQL.

CREATE TABLE IF NOT EXISTS discovered_instances (
    peer_id TEXT PRIMARY KEY,
    instance_name TEXT,
    instance_version TEXT,
    addresses_json TEXT NOT NULL,
    first_seen TEXT NOT NULL,
    last_seen TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_discovered_instances_last_seen
    ON discovered_instances(last_seen);
