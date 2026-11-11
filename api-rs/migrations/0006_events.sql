-- Activity feed for the website's live panel (not a PRD route): what happened, never who.
-- Identifiers are kept only in masked form (98XXXXX210), with no reporter or connection fields;
-- rejections keep the reason, never the value. Repeats, bursts, rejections and rate limits are
-- grouped into one row whose count grows. Cleared on reset.
-- Owner: B. Filled in by: B9.

CREATE TABLE events (
    id               INTEGER PRIMARY KEY,
    at               TEXT NOT NULL,          -- UTC, when it last happened
    kind             TEXT NOT NULL,          -- report, merged, burst, rejected, rate_limited,
                                             -- known_scam, dispute, alert
    pincode          TEXT,
    scam_type        TEXT,
    channel          TEXT,
    identifier_type  TEXT,
    display          TEXT,                   -- always masked
    case_id          TEXT,
    detail           TEXT,                   -- rejection reason or alert type, never input
    count            INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX events_by_time ON events (at);
CREATE INDEX events_by_group ON events (kind, case_id, detail, at);
