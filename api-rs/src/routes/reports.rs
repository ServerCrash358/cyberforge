//! `POST /reports`: validate, normalise, HMAC reporter fields, rate limit (429), store.
//! Replies 201 `{report_id, case_id, merged}`, 400, or 429 for a reporter over the hourly limit.

use crate::activity::{self, Context};
use crate::body::{Fields, json_object};
use crate::core::identity::connection_prefix;
use crate::core::normalise::{IdentifierType, normalise};
use crate::core::notes::sketch;
use crate::core::trust_config::RATE_LIMIT_PER_HOUR;
use crate::core::verified::is_verified;
use crate::core::{areas, isotime, pytext, validate};
use crate::errors::ApiError;
use crate::state::AppState;
use crate::store::{self, NewReport};
use axum::Json;
use axum::body::Bytes;
use axum::extract::{ConnectInfo, State};
use axum::http::StatusCode;
use axum::http::request::Parts;
use serde::Serialize;
use std::net::SocketAddr;
use time::OffsetDateTime;

/// PRD Appendix A section 2. Unknown extra fields are ignored, never stored.
struct ReportIn {
    report_id: String,
    occurred_at: String,
    reported_at: Option<String>,
    reporter_id: String,
    ip_prefix: Option<String>,
    channel: String,
    scam_type: String,
    identifier_type: IdentifierType,
    identifier_value: String,
    pincode: String,
    note: String,
    has_screenshot: bool,
    amount_lost_inr: f64,
}

fn read(parts: &Parts, bytes: &[u8]) -> Result<ReportIn, ApiError> {
    let map = json_object(&parts.headers, bytes)?;
    let mut f = Fields::new(&map);
    let report_id = f.required_str("report_id", 1);
    let occurred_at = f.required_str("occurred_at", 1);
    let reported_at = f.optional_str("reported_at");
    let reporter_id = f.required_str("reporter_id", 1);
    let ip_prefix = f.optional_str("ip_prefix");
    let channel = f.required_str("channel", 0);
    let scam_type = f.required_str("scam_type", 0);
    let identifier_type = f.required("identifier_type", IdentifierType::parse);
    let identifier_value = f.required_str("identifier_value", 0);
    let pincode = f.required_str("pincode", 0);
    f.optional_str("locality");
    f.optional_str("city");
    let note = f.str_or("note", "");
    let has_screenshot = f.bool_or("has_screenshot", false);
    let amount_lost_inr = f.non_negative_or("amount_lost_inr", 0.0);
    f.finish()?;
    Ok(ReportIn {
        report_id,
        occurred_at,
        reported_at,
        reporter_id,
        ip_prefix,
        channel,
        scam_type,
        identifier_type: identifier_type.expect("finish() refuses a missing type"),
        identifier_value,
        pincode,
        note,
        has_screenshot,
        amount_lost_inr,
    })
}

#[derive(Serialize)]
pub struct ReportOut {
    report_id: String,
    case_id: String,
    merged: bool,
}

enum Outcome {
    Stored(ReportOut),
    RateLimited,
}

pub async fn create(
    State(state): State<AppState>,
    parts: Parts,
    bytes: Bytes,
) -> Result<(StatusCode, Json<ReportOut>), ApiError> {
    let body = read(&parts, &bytes)?;
    let now = state.clock.now();
    let kind = body.identifier_type;

    let Ok(normalised) = normalise(kind, &body.identifier_value) else {
        rejected(&state, now, &body, &format!("invalid {}", kind.as_str()))?;
        return Err(ApiError::bad_request(format!(
            "invalid identifier_value for type {}",
            kind.as_str()
        )));
    };
    let checked = validate::occurred_at(&body.occurred_at, now)
        .and_then(|occurred| Ok((occurred, validate::pincode(&body.pincode)?)));
    let (occurred, pincode) = match checked {
        Ok(found) => found,
        Err(e) => {
            rejected(&state, now, &body, reason(&e.0))?;
            return Err(ApiError::bad_request(e.0));
        }
    };

    let reported = validate::reported_at(body.reported_at.as_deref(), occurred, now);
    let given_prefix = body.ip_prefix.as_deref().filter(|p| !p.is_empty());
    let prefix = given_prefix.map_or_else(
        || connection_prefix(client_address(&state, &parts).as_deref()),
        str::to_owned,
    );
    let keys = &state.keys;
    let new = NewReport {
        report_id: body.report_id.clone(),
        identifier_type: kind.as_str(),
        identifier_hmac: keys.identifier(kind.as_str(), &normalised),
        reporter_hmac: keys.reporter(&body.reporter_id),
        ip_prefix_hmac: keys.ip_prefix(&prefix),
        channel: validate::channel(&body.channel),
        scam_type: validate::scam_type(&body.scam_type),
        pincode: pincode.clone(),
        occurred_at: isotime::to_utc_text(occurred),
        reported_at: isotime::to_utc_text(reported),
        note_length: pytext::strip(&body.note).chars().count(),
        note_sketch: sketch(&body.note, &keys.note_key()),
        has_screenshot: body.has_screenshot,
        amount_lost_inr: body.amount_lost_inr,
        origin: if given_prefix.is_some() { "api" } else { "web" },
    };
    // Incidents 30 days old or more never show as activity.
    let fresh = !activity::too_old(occurred, now);
    let ctx = Context {
        occurred: Some(occurred),
        pincode: Some(pincode),
        scam_type: Some(new.scam_type.clone()),
        channel: Some(new.channel.clone()),
        identifier_type: Some(kind.as_str()),
        display: Some(activity::masked(kind, &normalised)),
        ..Context::at(now)
    };
    let verified = is_verified(kind, &normalised);

    let outcome = state.db.write(|tx| {
        if store::recent_reports(tx, &new.reporter_hmac, reported)? >= RATE_LIMIT_PER_HOUR {
            if fresh {
                activity::record(tx, "rate_limited", &ctx, None, true, None)?;
            }
            return Ok::<_, rusqlite::Error>(Outcome::RateLimited);
        }
        let before = store::find_case(tx, new.identifier_type, &new.identifier_hmac)?;
        let (case_id, merged, status) = store::add_report(tx, &new)?;
        let after = store::find_case(tx, new.identifier_type, &new.identifier_hmac)?;
        if let Some(after) = after.as_ref().filter(|_| fresh) {
            let ctx = Context {
                case_id: Some(case_id.clone()),
                ..ctx.clone()
            };
            activity::report_events(tx, &ctx, &status, before.as_ref(), after, verified)?;
        }
        Ok(Outcome::Stored(ReportOut {
            report_id: body.report_id.clone(),
            case_id,
            merged,
        }))
    })?;
    match outcome {
        Outcome::Stored(out) => Ok((StatusCode::CREATED, Json(out))),
        Outcome::RateLimited => Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "too many reports from this reporter; try again later",
        )),
    }
}

/// The caller's address, for reports with no `ip_prefix` (the website's). Behind Caddy, the
/// connection comes from the proxy, so the address is the last `X-Forwarded-For` entry, the one
/// Caddy wrote; earlier entries come from the client and are not trusted.
fn client_address(state: &AppState, parts: &Parts) -> Option<String> {
    if state.options.trust_proxy {
        let forwarded = parts.headers.get("x-forwarded-for")?.to_str().ok()?;
        return forwarded.rsplit(',').next().map(|ip| ip.trim().to_owned());
    }
    let peer = parts.extensions.get::<ConnectInfo<SocketAddr>>()?;
    Some(peer.0.ip().to_string())
}

/// A short reason for the activity feed, from a rejection message that holds no input.
fn reason(message: &str) -> &'static str {
    if message.starts_with("invalid pincode") {
        "unknown area"
    } else if message.contains("future") {
        "time in the future"
    } else {
        "invalid time"
    }
}

/// Shows a rejection in the feed: the reason and, for a real area, where. Never the value.
fn rejected(
    state: &AppState,
    now: OffsetDateTime,
    body: &ReportIn,
    why: &str,
) -> Result<(), ApiError> {
    if let Ok(occurred) = validate::parse_time(&body.occurred_at, "occurred_at")
        && activity::too_old(occurred, now)
    {
        return Ok(());
    }
    let pincode = pytext::strip(&body.pincode);
    let place = Context {
        pincode: areas::is_known(pincode).then(|| pincode.to_owned()),
        ..Context::at(now)
    };
    state
        .db
        .write(|tx| activity::record(tx, "rejected", &place, Some(why), true, None))?;
    Ok(())
}
