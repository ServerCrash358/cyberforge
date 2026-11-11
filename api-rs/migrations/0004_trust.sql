-- Trust rules (task B5): note fingerprints, clusters, weights and first-time flags per report.
-- reported_at is now the report's own reported_at, clamped between occurred_at and server time.
-- Owner: B. Filled in by: B5.

ALTER TABLE reports ADD COLUMN note_sketch BLOB NOT NULL DEFAULT x'';
ALTER TABLE reports ADD COLUMN cluster_id INTEGER;
ALTER TABLE reports ADD COLUMN weight REAL NOT NULL DEFAULT 0;
ALTER TABLE reports ADD COLUMN first_time INTEGER NOT NULL DEFAULT 1;

UPDATE reports SET cluster_id = id WHERE cluster_id IS NULL;

CREATE INDEX reports_by_reporter ON reports (reporter_hmac, reported_at);
