-- Message embed persistence and URL cache for Story 6.7.
-- Compatible with SQLite + PostgreSQL.

CREATE TABLE IF NOT EXISTS embed_url_cache (
    normalized_url TEXT PRIMARY KEY,
    title TEXT NULL,
    description TEXT NULL,
    thumbnail_url TEXT NULL,
    domain TEXT NOT NULL,
    fetched_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CONSTRAINT ck_embed_url_cache_normalized_url_non_empty CHECK (LENGTH(TRIM(normalized_url)) > 0),
    CONSTRAINT ck_embed_url_cache_domain_non_empty CHECK (LENGTH(TRIM(domain)) > 0)
);

CREATE TABLE IF NOT EXISTS message_embeds (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    url TEXT NOT NULL,
    normalized_url TEXT NOT NULL,
    title TEXT NULL,
    description TEXT NULL,
    thumbnail_url TEXT NULL,
    domain TEXT NOT NULL,
    created_at TEXT NOT NULL,
    CONSTRAINT ck_message_embeds_url_non_empty CHECK (LENGTH(TRIM(url)) > 0),
    CONSTRAINT ck_message_embeds_normalized_url_non_empty CHECK (LENGTH(TRIM(normalized_url)) > 0),
    CONSTRAINT ck_message_embeds_domain_non_empty CHECK (LENGTH(TRIM(domain)) > 0)
);

CREATE INDEX IF NOT EXISTS idx_message_embeds_message_id_created_at
    ON message_embeds(message_id, created_at);

CREATE INDEX IF NOT EXISTS idx_message_embeds_normalized_url
    ON message_embeds(normalized_url);
