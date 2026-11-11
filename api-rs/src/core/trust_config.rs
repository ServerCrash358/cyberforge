//! Every trust threshold and weight in one place, the same values as `api/app/core/trust_config.py`.
//! Change them only after a run on the judge simulator's tuning seeds; report on held-out seeds.

use time::Duration;

/// PRD privacy rule 2: hide the value until at least 3 different people reported it.
pub const UNMASK_MIN_REPORTERS: usize = 3;

/// Rule 6: `KNOWN_SCAM` needs 3 independent clusters and a weighted score of 3.0.
pub const KNOWN_SCAM_MIN_CLUSTERS: usize = 3;
pub const KNOWN_SCAM_MIN_SCORE: f64 = 3.0;

/// Rule 1: more than this many reports from one reporter within an hour gets 429.
pub const RATE_LIMIT_PER_HOUR: usize = 3;
pub const RATE_LIMIT_WINDOW: Duration = Duration::hours(1);

/// Rule 2: reports from one `ip_prefix` this close together are one person.
pub const SAME_CONNECTION_WINDOW: Duration = Duration::hours(24);

/// Rule 3: notes at least this similar are one voice (bottom-k `MinHash` of 5-character pieces).
pub const NOTE_DUPLICATE_SIMILARITY: f64 = 0.8;
pub const NOTE_SHINGLE_CHARS: usize = 5;
pub const NOTE_SKETCH_SIZE: usize = 128;

/// Rule 4: a coordinated burst of first-time reporters weighs at most `BURST_MAX_WEIGHT` and
/// counts as one independent cluster.
pub const BURST_WINDOW: Duration = Duration::hours(1);
pub const BURST_MIN_REPORTS: usize = 5;
pub const BURST_MOSTLY: f64 = 0.6;
pub const BURST_INCIDENT_SPAN: Duration = Duration::minutes(15);
pub const BURST_MAX_WEIGHT: f64 = 1.5;

/// Rule 5: a cluster in its own words weighs more than one with an empty or very short note.
pub const OWN_WORDS_MIN_CHARS: usize = 15;
pub const WEIGHT_OWN_WORDS: f64 = 1.0;
pub const WEIGHT_SHORT_NOTE: f64 = 0.6;

/// Lookup evidence: the most recent reports shown, newest first.
pub const EVIDENCE_LIMIT: usize = 5;
