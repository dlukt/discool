-- Track verification endpoint attempts per requester IP for rate limiting.
-- Compatible with SQLite and PostgreSQL.

CREATE TABLE IF NOT EXISTS email_verification_attempts (
    requester_ip TEXT NOT NULL,
    attempted_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_email_verification_attempts_ip_attempted_at
    ON email_verification_attempts(requester_ip, attempted_at);
