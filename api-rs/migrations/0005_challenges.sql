-- Challenges (task B6): a business disputes an identifier. The claimant's name and email are
-- kept for review only: never returned by any route, never logged, cleared on reset.
-- Owner: B. Filled in by: B6.

CREATE TABLE challenges (
    challenge_id     TEXT PRIMARY KEY,
    identifier_type  TEXT NOT NULL,
    identifier_hmac  TEXT NOT NULL,
    claimant_name    TEXT NOT NULL,
    contact_email    TEXT NOT NULL,
    proof_note       TEXT NOT NULL,
    status           TEXT NOT NULL DEFAULT 'under_review',
    created          TEXT NOT NULL
);

CREATE INDEX reports_by_area_status ON reports (pincode, status, occurred_at);
