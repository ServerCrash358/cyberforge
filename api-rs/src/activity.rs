//! The activity feed behind the website's live panel: what happened, never who.
//!
//! Every accepted report, repeat, coordinated burst, rejection, rate limit, new known scam,
//! dispute and alert becomes an event. Identifiers appear only masked; no reporter or connection
//! field is ever recorded; rejections keep their reason, never the value sent. Repeats, bursts,
//! rejections and rate limits within an hour are grouped into one event whose count grows.
//!
//! Nothing older than 30 days is ever shown: a report whose incident is that old records no
//! event at all, and an event stops appearing once its incident turns 30 days old.

use crate::core::alerts::{self as alert_rules, Alert};
use crate::core::areas;
use crate::core::isotime::{to_display, to_utc_text};
use crate::core::mask::display_value;
use crate::core::normalise::IdentifierType;
use crate::core::verdict::{CaseFacts, Verdict, verdict};
use crate::store::{self, CaseRow, sql_int};
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use time::{Duration, OffsetDateTime};

pub const GROUP_WINDOW: Duration = Duration::hours(1);
pub const FEED_LIMIT: usize = 40;
pub const MAX_AGE: Duration = Duration::days(30);

/// True when the incident is 30 days old or more, so it must not show as activity.
pub fn too_old(when: OffsetDateTime, now: OffsetDateTime) -> bool {
    when <= now - MAX_AGE
}

/// What an event may say about a report: area, type, channel and the masked identifier.
#[derive(Debug, Clone, Default)]
pub struct Context {
    pub at: Option<OffsetDateTime>,
    /// When the incident happened; defaults to `at`.
    pub occurred: Option<OffsetDateTime>,
    pub pincode: Option<String>,
    pub scam_type: Option<String>,
    pub channel: Option<String>,
    pub identifier_type: Option<&'static str>,
    pub display: Option<String>,
    pub case_id: Option<String>,
}

impl Context {
    pub fn at(at: OffsetDateTime) -> Self {
        Self {
            at: Some(at),
            ..Self::default()
        }
    }

    fn when(&self) -> OffsetDateTime {
        self.at.expect("every event has a time")
    }
}

/// Always the masked form, whatever the reporter count: events never hold raw values.
pub fn masked(kind: IdentifierType, normalised: &str) -> String {
    display_value(kind, normalised, 0)
}

/// Adds an event, or with `group` grows the matching one from the last hour.
///
/// # Errors
/// Any database error.
pub fn record(
    conn: &Connection,
    kind: &str,
    ctx: &Context,
    detail: Option<&str>,
    group: bool,
    count: Option<usize>,
) -> rusqlite::Result<()> {
    let at = to_utc_text(ctx.when());
    let occurred = to_utc_text(ctx.occurred.unwrap_or(ctx.when()));
    if group {
        let since = to_utc_text(ctx.when() - GROUP_WINDOW);
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM events WHERE kind = ? AND case_id IS ? AND detail IS ? AND at > ? \
                 ORDER BY at DESC LIMIT 1",
                params![kind, ctx.case_id, detail, since],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(id) = existing {
            conn.execute(
                "UPDATE events SET at = ?, count = COALESCE(?, count + 1), \
                 occurred_at = MAX(occurred_at, ?) WHERE id = ?",
                params![at, count.map(sql_int), occurred, id],
            )?;
            return Ok(());
        }
    }
    conn.execute(
        "INSERT INTO events (at, kind, pincode, scam_type, channel, identifier_type, display, \
         case_id, detail, count, occurred_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            at,
            kind,
            ctx.pincode,
            ctx.scam_type,
            ctx.channel,
            ctx.identifier_type,
            ctx.display,
            ctx.case_id,
            detail,
            sql_int(count.unwrap_or(1)),
            occurred,
        ],
    )?;
    Ok(())
}

fn case_verdict(case: Option<&CaseRow>, verified: bool) -> Verdict {
    let facts = case.map_or(
        CaseFacts {
            report_count: 0,
            clusters: 0,
            weighted_score: 0.0,
            challenged: false,
        },
        |c| CaseFacts {
            report_count: c.report_count,
            clusters: c.distinct_reporters,
            weighted_score: c.weighted_score,
            challenged: c.challenged,
        },
    );
    verdict(facts, verified)
}

/// Events for one accepted report: how it was counted, a burst, a new verdict, new alerts.
///
/// # Errors
/// Any database error.
pub fn report_events(
    conn: &Connection,
    ctx: &Context,
    status: &str,
    before: Option<&CaseRow>,
    after: &CaseRow,
    verified: bool,
) -> rusqlite::Result<()> {
    match status {
        "counted" => record(conn, "report", ctx, None, false, None)?,
        "merged" => record(conn, "merged", ctx, None, true, None)?,
        _ => {}
    }
    let suspects = store::suspect_count(conn, &after.case_id)?;
    if suspects > 0 {
        record(conn, "burst", ctx, None, true, Some(suspects))?;
    }
    if case_verdict(Some(after), verified) == Verdict::KnownScam
        && case_verdict(before, verified) != Verdict::KnownScam
    {
        record(conn, "known_scam", ctx, None, false, None)?;
    }
    if let Some(pincode) = &ctx.pincode {
        new_alerts(conn, ctx, pincode)?;
    }
    Ok(())
}

fn alert_parts(alert: &Alert) -> (&'static str, Option<&str>) {
    match alert {
        Alert::Spike { .. } => ("SPIKE", None),
        Alert::NewType { scam_type, .. } => ("NEW_TYPE", Some(scam_type)),
    }
}

/// Announces each alert for the area once a day, when it first appears.
fn new_alerts(conn: &Connection, ctx: &Context, pincode: &str) -> rusqlite::Result<()> {
    let now = ctx.when();
    let events = store::area_events(conn, pincode, &to_utc_text(now - alert_rules::HISTORY))?;
    for alert in alert_rules::alerts(pincode, &events, now) {
        let (kind, scam_type) = alert_parts(&alert);
        let seen: bool = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM events WHERE kind = 'alert' AND pincode = ? \
             AND detail = ? AND scam_type IS ? AND at > ?)",
            params![
                pincode,
                kind,
                scam_type,
                to_utc_text(now - Duration::hours(24))
            ],
            |r| r.get(0),
        )?;
        if !seen {
            let place = Context {
                pincode: Some(pincode.to_owned()),
                scam_type: scam_type.map(str::to_owned),
                ..Context::at(now)
            };
            record(conn, "alert", &place, Some(kind), false, None)?;
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct FeedEvent {
    pub at: String,
    pub kind: String,
    pub pincode: Option<String>,
    pub locality: Option<String>,
    pub scam_type: Option<String>,
    pub channel: Option<String>,
    pub identifier_type: Option<String>,
    pub display: Option<String>,
    pub detail: Option<String>,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct LiveAlert {
    #[serde(flatten)]
    pub alert: Alert,
    pub locality: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Feed {
    pub events: Vec<FeedEvent>,
    pub alerts: Vec<LiveAlert>,
}

fn locality(pincode: Option<&str>) -> Option<String> {
    pincode.and_then(areas::get).map(|a| a.locality.clone())
}

/// The newest events from the last 30 days, and the alerts live right now in every area.
///
/// # Errors
/// Any database error.
pub fn feed(conn: &Connection, now: OffsetDateTime) -> rusqlite::Result<Feed> {
    let mut stmt = conn.prepare(
        "SELECT at, kind, pincode, scam_type, channel, identifier_type, display, detail, count \
         FROM events WHERE occurred_at > ? ORDER BY at DESC, id DESC LIMIT ?",
    )?;
    let events = stmt
        .query_map(
            params![to_utc_text(now - MAX_AGE), sql_int(FEED_LIMIT)],
            |r| {
                let pincode: Option<String> = r.get(2)?;
                Ok(FeedEvent {
                    at: to_display(&r.get::<_, String>(0)?),
                    kind: r.get(1)?,
                    locality: locality(pincode.as_deref()),
                    pincode,
                    scam_type: r.get(3)?,
                    channel: r.get(4)?,
                    identifier_type: r.get(5)?,
                    display: r.get(6)?,
                    detail: r.get(7)?,
                    count: usize::try_from(r.get::<_, i64>(8)?).unwrap_or(0),
                })
            },
        )?
        .collect::<rusqlite::Result<_>>()?;
    let since = to_utc_text(now - alert_rules::HISTORY);
    let mut alerts = Vec::new();
    for pincode in store::active_areas(conn, &to_utc_text(now - alert_rules::WINDOW))? {
        let area_events = store::area_events(conn, &pincode, &since)?;
        for alert in alert_rules::alerts(&pincode, &area_events, now) {
            alerts.push(LiveAlert {
                alert,
                locality: locality(Some(&pincode)),
            });
        }
    }
    Ok(Feed { events, alerts })
}
