//! SQL for reports, cases and challenges: the only module that reads or writes those tables.
//!
//! Every value passed in is already validated, normalised and, for identifiers and reporter
//! fields, a keyed HMAC.

use crate::core::isotime;
use crate::core::trust::{ReportFacts, assess};
use crate::core::trust_config::RATE_LIMIT_WINDOW;
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::HashMap;
use time::OffsetDateTime;

pub struct NewReport {
    pub report_id: String,
    pub identifier_type: &'static str,
    pub identifier_hmac: String,
    pub reporter_hmac: String,
    pub ip_prefix_hmac: String,
    pub channel: String,
    pub scam_type: String,
    pub pincode: String,
    pub occurred_at: String,
    pub reported_at: String,
    pub note_length: usize,
    pub note_sketch: Vec<u8>,
    pub has_screenshot: bool,
    pub amount_lost_inr: f64,
    pub origin: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseRow {
    pub case_id: String,
    pub report_count: usize,
    pub distinct_reporters: usize,
    pub weighted_score: f64,
    pub first_seen: Option<String>,
    pub last_seen: Option<String>,
    pub top_scam_type: Option<String>,
    pub challenged: bool,
}

pub struct EvidenceRow {
    pub occurred_at: String,
    pub channel: String,
    pub scam_type: String,
    pub pincode: String,
}

/// A random id like `case_1f2e3d4c5b6a7980`, as the Python API made them.
///
/// # Panics
/// When the operating system cannot supply random bytes.
pub fn new_id(prefix: &str) -> String {
    let mut bytes = [0u8; 8];
    getrandom::fill(&mut bytes).expect("the operating system provides random bytes");
    format!("{prefix}_{}", hex::encode(bytes))
}

fn count(n: i64) -> usize {
    usize::try_from(n).unwrap_or(0)
}

/// A count as SQLite stores it (rusqlite has no `usize` parameters).
pub fn sql_int(n: usize) -> i64 {
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// Stores a report under its identifier's case. Returns (`case_id`, merged, its trust status).
///
/// # Errors
/// Any database error.
pub fn add_report(
    conn: &Connection,
    report: &NewReport,
) -> rusqlite::Result<(String, bool, String)> {
    let existing: Option<String> = conn
        .query_row(
            "SELECT case_id FROM cases WHERE identifier_type = ? AND identifier_hmac = ?",
            params![report.identifier_type, report.identifier_hmac],
            |r| r.get(0),
        )
        .optional()?;
    let merged = existing.is_some();
    let case_id = existing.unwrap_or_else(|| new_id("case"));
    if !merged {
        conn.execute(
            "INSERT INTO cases (case_id, identifier_type, identifier_hmac) VALUES (?, ?, ?)",
            params![case_id, report.identifier_type, report.identifier_hmac],
        )?;
    }
    let seen_before: bool = conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM reports WHERE reporter_hmac = ? AND reported_at < ?)",
        params![report.reporter_hmac, report.reported_at],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO reports (report_id, case_id, reporter_hmac, ip_prefix_hmac, channel, \
         scam_type, pincode, occurred_at, reported_at, note_length, note_sketch, \
         has_screenshot, amount_lost_inr, origin, first_time) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            report.report_id,
            case_id,
            report.reporter_hmac,
            report.ip_prefix_hmac,
            report.channel,
            report.scam_type,
            report.pincode,
            report.occurred_at,
            report.reported_at,
            sql_int(report.note_length),
            report.note_sketch,
            report.has_screenshot,
            report.amount_lost_inr,
            report.origin,
            !seen_before,
        ],
    )?;
    let row_id = conn.last_insert_rowid();
    recompute(conn, &case_id)?;
    let status: String =
        conn.query_row("SELECT status FROM reports WHERE id = ?", [row_id], |r| {
            r.get(0)
        })?;
    Ok((case_id, merged, status))
}

/// # Errors
/// Any database error.
pub fn suspect_count(conn: &Connection, case_id: &str) -> rusqlite::Result<usize> {
    conn.query_row(
        "SELECT COUNT(*) FROM reports WHERE case_id = ? AND status = 'suspect'",
        [case_id],
        |r| r.get(0).map(count),
    )
}

/// Areas with counted reports since a time: the only places an alert can be live.
///
/// # Errors
/// Any database error.
pub fn active_areas(conn: &Connection, since: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT pincode FROM reports WHERE status = 'counted' AND occurred_at > ? \
         ORDER BY pincode",
    )?;
    stmt.query_map([since], |r| r.get(0))?.collect()
}

/// How many reports this reporter made within the rate-limit window around this one.
///
/// # Errors
/// Any database error.
pub fn recent_reports(
    conn: &Connection,
    reporter_hmac: &str,
    reported_at: OffsetDateTime,
) -> rusqlite::Result<usize> {
    let low = isotime::to_utc_text(reported_at - RATE_LIMIT_WINDOW);
    let high = isotime::to_utc_text(reported_at + RATE_LIMIT_WINDOW);
    conn.query_row(
        "SELECT COUNT(*) FROM reports WHERE reporter_hmac = ? AND reported_at > ? \
         AND reported_at < ?",
        params![reporter_hmac, low, high],
        |r| r.get(0).map(count),
    )
}

fn stored_time(text: &str) -> OffsetDateTime {
    isotime::parse(text).expect("stored times are valid")
}

/// Re-runs the trust rules over the whole case and refreshes what a lookup reads.
fn recompute(conn: &Connection, case_id: &str) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        "SELECT id, reporter_hmac, ip_prefix_hmac, reported_at, occurred_at, note_length, \
         note_sketch, first_time FROM reports WHERE case_id = ?",
    )?;
    let facts: Vec<ReportFacts> = stmt
        .query_map([case_id], |r| {
            Ok(ReportFacts {
                id: r.get(0)?,
                reporter: r.get(1)?,
                ip: r.get(2)?,
                reported_at: stored_time(&r.get::<_, String>(3)?),
                occurred_at: stored_time(&r.get::<_, String>(4)?),
                note_length: count(r.get(5)?),
                sketch: r.get(6)?,
                first_time: r.get(7)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    let result = assess(&facts);
    let mut update =
        conn.prepare("UPDATE reports SET status = ?, cluster_id = ?, weight = ? WHERE id = ?")?;
    for (id, status) in &result.statuses {
        update.execute(params![
            status.as_str(),
            result.cluster_of[id],
            result.weight_of[id],
            id
        ])?;
    }
    let (n, first, last): (i64, Option<String>, Option<String>) = conn.query_row(
        "SELECT COUNT(*), MIN(occurred_at), MAX(occurred_at) FROM reports WHERE case_id = ?",
        [case_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let top: Option<String> = conn
        .query_row(
            "SELECT scam_type FROM reports WHERE case_id = ? GROUP BY scam_type \
             ORDER BY COUNT(*) DESC, MAX(occurred_at) DESC LIMIT 1",
            [case_id],
            |r| r.get(0),
        )
        .optional()?;
    conn.execute(
        "UPDATE cases SET report_count = ?, distinct_reporters = ?, weighted_score = ?, \
         first_seen = ?, last_seen = ?, top_scam_type = ? WHERE case_id = ?",
        params![
            n,
            sql_int(result.clusters),
            result.score,
            first,
            last,
            top,
            case_id
        ],
    )?;
    Ok(())
}

/// # Errors
/// Any database error.
pub fn find_case(
    conn: &Connection,
    identifier_type: &str,
    identifier_hmac: &str,
) -> rusqlite::Result<Option<CaseRow>> {
    conn.query_row(
        "SELECT case_id, report_count, distinct_reporters, weighted_score, first_seen, \
         last_seen, top_scam_type, challenged FROM cases \
         WHERE identifier_type = ? AND identifier_hmac = ?",
        params![identifier_type, identifier_hmac],
        |r| {
            Ok(CaseRow {
                case_id: r.get(0)?,
                report_count: count(r.get(1)?),
                distinct_reporters: count(r.get(2)?),
                weighted_score: r.get(3)?,
                first_seen: r.get(4)?,
                last_seen: r.get(5)?,
                top_scam_type: r.get(6)?,
                challenged: r.get(7)?,
            })
        },
    )
    .optional()
}

/// The newest reports of a case, newest first.
///
/// # Errors
/// Any database error.
pub fn evidence(
    conn: &Connection,
    case_id: &str,
    limit: usize,
) -> rusqlite::Result<Vec<EvidenceRow>> {
    let mut stmt = conn.prepare(
        "SELECT occurred_at, channel, scam_type, pincode FROM reports WHERE case_id = ? \
         ORDER BY occurred_at DESC LIMIT ?",
    )?;
    stmt.query_map(params![case_id, sql_int(limit)], |r| {
        Ok(EvidenceRow {
            occurred_at: r.get(0)?,
            channel: r.get(1)?,
            scam_type: r.get(2)?,
            pincode: r.get(3)?,
        })
    })?
    .collect()
}

/// Counted reports per area with `occurred_at` in (since, until]: (count, top scam type).
///
/// # Errors
/// Any database error.
///
/// # Panics
/// Never: a list of strings always serialises.
pub fn area_counts(
    conn: &Connection,
    pincodes: &[&str],
    since: &str,
    until: &str,
) -> rusqlite::Result<HashMap<String, (usize, String)>> {
    if pincodes.is_empty() {
        return Ok(HashMap::new());
    }
    let list = serde_json::to_string(pincodes).expect("strings serialise");
    let mut stmt = conn.prepare(
        "SELECT pincode, scam_type, COUNT(*), MAX(occurred_at) FROM reports \
         WHERE status = 'counted' AND pincode IN (SELECT value FROM json_each(?)) \
         AND occurred_at > ? AND occurred_at <= ? GROUP BY pincode, scam_type",
    )?;
    let rows = stmt.query_map(params![list, since, until], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            count(r.get(2)?),
            r.get::<_, String>(3)?,
        ))
    })?;
    let mut totals: HashMap<String, usize> = HashMap::new();
    let mut best: HashMap<String, (usize, String, String)> = HashMap::new();
    for row in rows {
        let (pin, kind, n, latest) = row?;
        *totals.entry(pin.clone()).or_default() += n;
        let better = best
            .get(&pin)
            .is_none_or(|(bn, bl, _)| (n, &latest) > (*bn, bl));
        if better {
            best.insert(pin, (n, latest, kind));
        }
    }
    Ok(totals
        .into_iter()
        .map(|(pin, total)| {
            let kind = best.remove(&pin).map(|(_, _, k)| k).unwrap_or_default();
            (pin, (total, kind))
        })
        .collect())
}

/// (`occurred_at`, `scam_type`) of counted reports in one area since a time.
///
/// # Errors
/// Any database error.
pub fn area_events(
    conn: &Connection,
    pincode: &str,
    since: &str,
) -> rusqlite::Result<Vec<(OffsetDateTime, String)>> {
    let mut stmt = conn.prepare(
        "SELECT occurred_at, scam_type FROM reports WHERE status = 'counted' AND pincode = ? \
         AND occurred_at > ?",
    )?;
    stmt.query_map(params![pincode, since], |r| {
        Ok((stored_time(&r.get::<_, String>(0)?), r.get(1)?))
    })?
    .collect()
}

/// Whether this identifier already has a dispute: only one is allowed per number.
///
/// # Errors
/// Any database error.
pub fn has_challenge(
    conn: &Connection,
    identifier_type: &str,
    identifier_hmac: &str,
) -> rusqlite::Result<bool> {
    conn.query_row(
        "SELECT EXISTS (SELECT 1 FROM challenges WHERE identifier_type = ? AND identifier_hmac = ?)",
        params![identifier_type, identifier_hmac],
        |r| r.get(0),
    )
}

pub struct NewChallenge<'a> {
    pub identifier_type: &'a str,
    pub identifier_hmac: &'a str,
    pub claimant_name: &'a str,
    pub contact_email: &'a str,
    pub proof_note: &'a str,
    pub created: &'a str,
}

/// Records a dispute and marks the identifier's case (creating it if needed) as challenged.
///
/// # Errors
/// Any database error.
pub fn add_challenge(conn: &Connection, c: &NewChallenge<'_>) -> rusqlite::Result<String> {
    let challenge_id = new_id("chl");
    conn.execute(
        "INSERT INTO challenges (challenge_id, identifier_type, identifier_hmac, claimant_name, \
         contact_email, proof_note, created) VALUES (?, ?, ?, ?, ?, ?, ?)",
        params![
            challenge_id,
            c.identifier_type,
            c.identifier_hmac,
            c.claimant_name,
            c.contact_email,
            c.proof_note,
            c.created
        ],
    )?;
    let updated = conn.execute(
        "UPDATE cases SET challenged = 1 WHERE identifier_type = ? AND identifier_hmac = ?",
        params![c.identifier_type, c.identifier_hmac],
    )?;
    if updated == 0 {
        conn.execute(
            "INSERT INTO cases (case_id, identifier_type, identifier_hmac, challenged) \
             VALUES (?, ?, ?, 1)",
            params![new_id("case"), c.identifier_type, c.identifier_hmac],
        )?;
    }
    Ok(challenge_id)
}

/// Deletes every report, case, challenge and event. Reference data and the licence are kept.
///
/// # Errors
/// Any database error.
pub fn reset(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "DELETE FROM events; DELETE FROM reports; DELETE FROM challenges; DELETE FROM cases;",
    )
}
