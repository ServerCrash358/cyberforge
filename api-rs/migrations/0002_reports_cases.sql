-- Reports and cases (task B2). Identifiers and reporter fields are keyed HMACs only;
-- the note is reduced to its length here, and to a fingerprint in B5. No raw values.
-- Owner: B. Filled in by: B2.

CREATE TABLE cases (
    case_id             TEXT PRIMARY KEY,
    identifier_type     TEXT NOT NULL,
    identifier_hmac     TEXT NOT NULL,
    report_count        INTEGER NOT NULL DEFAULT 0,
    distinct_reporters  INTEGER NOT NULL DEFAULT 0,
    weighted_score      REAL NOT NULL DEFAULT 0,
    first_seen          TEXT,
    last_seen           TEXT,
    top_scam_type       TEXT,
    challenged          INTEGER NOT NULL DEFAULT 0,
    UNIQUE (identifier_type, identifier_hmac)
);

CREATE TABLE reports (
    id                  INTEGER PRIMARY KEY,
    report_id           TEXT NOT NULL,
    case_id             TEXT NOT NULL REFERENCES cases (case_id) ON DELETE CASCADE,
    reporter_hmac       TEXT NOT NULL,
    ip_prefix_hmac      TEXT NOT NULL,
    channel             TEXT NOT NULL,
    scam_type           TEXT NOT NULL,
    pincode             TEXT NOT NULL,
    occurred_at         TEXT NOT NULL,  -- UTC, ISO 8601
    reported_at         TEXT NOT NULL,  -- UTC, ISO 8601
    note_length         INTEGER NOT NULL,
    has_screenshot      INTEGER NOT NULL,
    amount_lost_inr     REAL NOT NULL,
    status              TEXT NOT NULL DEFAULT 'counted'
                        CHECK (status IN ('counted', 'merged', 'suspect')),
    origin              TEXT NOT NULL
);

CREATE INDEX reports_by_case ON reports (case_id, occurred_at);
CREATE INDEX reports_by_area ON reports (pincode, occurred_at);
