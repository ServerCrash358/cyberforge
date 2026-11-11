-- Activity events remember when the thing they describe happened (task B9), so the live panel can
-- hide anything older than 30 days, even when a report about it was only filed today.
-- Existing events take the newest occurrence in their case, or their own time when they have none.
-- Owner: B. Filled in by: B9.

ALTER TABLE events ADD COLUMN occurred_at TEXT;

UPDATE events SET occurred_at = COALESCE(
    (SELECT MAX(r.occurred_at) FROM reports r
      WHERE r.case_id = events.case_id
        AND events.kind IN ('report', 'merged', 'burst', 'known_scam')),
    at
);

CREATE INDEX events_by_occurrence ON events (occurred_at);
