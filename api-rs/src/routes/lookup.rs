//! `GET /lookup?type&value`: verdict, counts, masked `display_value`, evidence, challenged.
//! Unreported is 200 `NO_REPORTS`; invalid is 400.

use crate::body::{Fields, query_object};
use crate::core::isotime::to_display;
use crate::core::mask::display_value;
use crate::core::normalise::{IdentifierType, normalise};
use crate::core::trust_config::EVIDENCE_LIMIT;
use crate::core::verdict::{CaseFacts, Verdict, verdict};
use crate::core::verified::is_verified;
use crate::errors::ApiError;
use crate::state::AppState;
use crate::store;
use axum::Json;
use axum::extract::{RawQuery, State};
use serde::Serialize;

#[derive(Serialize)]
pub struct Evidence {
    occurred_at: String,
    channel: String,
    scam_type: String,
    pincode: String,
}

#[derive(Serialize)]
pub struct LookupOut {
    verdict: Verdict,
    report_count: usize,
    distinct_reporters: usize,
    first_seen: Option<String>,
    last_seen: Option<String>,
    scam_type: Option<String>,
    display_value: String,
    evidence: Vec<Evidence>,
    challenged: bool,
}

pub async fn lookup(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
) -> Result<Json<LookupOut>, ApiError> {
    let params = query_object(query.as_deref());
    let mut f = Fields::new(&params);
    let kind = f.required("type", IdentifierType::parse);
    let value = f.required_str("value", 1);
    f.finish()?;
    let kind = kind.expect("finish() refuses a missing type");
    let normalised = normalise(kind, &value)
        .map_err(|_| ApiError::bad_request(format!("invalid value for type {}", kind.as_str())))?;

    let verified = is_verified(kind, &normalised);
    let hmac = state.keys.identifier(kind.as_str(), &normalised);
    let (case, rows) = state.db.read(|conn| {
        let case = store::find_case(conn, kind.as_str(), &hmac)?;
        let rows = match &case {
            Some(c) => store::evidence(conn, &c.case_id, EVIDENCE_LIMIT)?,
            None => Vec::new(),
        };
        Ok::<_, rusqlite::Error>((case, rows))
    })?;

    let facts = case.as_ref().map_or(
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
    let reporters = case.as_ref().map_or(0, |c| c.distinct_reporters);
    Ok(Json(LookupOut {
        verdict: verdict(facts, verified),
        report_count: facts.report_count,
        distinct_reporters: reporters,
        first_seen: case
            .as_ref()
            .and_then(|c| c.first_seen.as_deref())
            .map(to_display),
        last_seen: case
            .as_ref()
            .and_then(|c| c.last_seen.as_deref())
            .map(to_display),
        scam_type: case.as_ref().and_then(|c| c.top_scam_type.clone()),
        // Verified values are public numbers; everything else stays masked below 3 people.
        display_value: if verified {
            normalised.clone()
        } else {
            display_value(kind, &normalised, reporters)
        },
        // Evidence on a verified value would only help an attacker tune a campaign.
        evidence: if verified {
            Vec::new()
        } else {
            rows.into_iter()
                .map(|r| Evidence {
                    occurred_at: to_display(&r.occurred_at),
                    channel: r.channel,
                    scam_type: r.scam_type,
                    pincode: r.pincode,
                })
                .collect()
        },
        challenged: facts.challenged,
    }))
}
