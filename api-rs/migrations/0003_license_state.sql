-- Licence state (task B3): the licence now in force and the latest time the server has seen.
-- One row. Kept on /admin/reset.
-- Owner: B. Filled in by: B3.

CREATE TABLE license_state (
    id              INTEGER PRIMARY KEY CHECK (id = 1),
    licence_json    TEXT,
    high_water      TEXT
);

INSERT INTO license_state (id, licence_json, high_water) VALUES (1, NULL, NULL);
