//! `POST /challenges`: a business disputes an identifier; 201 `{challenge_id, status}`.
//! While under review, lookups show `challenged: true` and are never `KNOWN_SCAM`.
//! The claimant's name and email are stored for review only: never returned or logged.

use crate::activity::{self, Context};
use crate::body::{Fields, json_object};
use crate::core::isotime::to_utc_text;
use crate::core::normalise::{IdentifierType, normalise};
use crate::core::pytext;
use crate::errors::ApiError;
use crate::state::AppState;
use crate::store::{self, NewChallenge};
use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use serde::Serialize;

#[derive(Serialize)]
pub struct ChallengeOut {
    challenge_id: String,
    status: &'static str,
}

enum Outcome {
    Created(String),
    AlreadyDisputed,
}

pub async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    bytes: Bytes,
) -> Result<(StatusCode, Json<ChallengeOut>), ApiError> {
    let map = json_object(&headers, &bytes)?;
    let mut f = Fields::new(&map);
    let kind = f.required("identifier_type", IdentifierType::parse);
    let value = f.required_str("identifier_value", 0);
    let claimant_name = f.bounded_str("claimant_name", 1, 200);
    let contact_email = f.bounded_str("contact_email", 3, 254);
    let proof_note = f.bounded_str("proof_note", 0, 2000);
    f.finish()?;
    let kind = kind.expect("finish() refuses a missing type");
    let normalised = normalise(kind, &value).map_err(|_| {
        ApiError::bad_request(format!(
            "invalid identifier_value for type {}",
            kind.as_str()
        ))
    })?;
    let identifier_hmac = state.keys.identifier(kind.as_str(), &normalised);
    let now = state.clock.now();

    // One dispute per number. The PRD allows 429 for slowing someone down, and the check and
    // the insert share one transaction, so two at once cannot both get through.
    let outcome = state.db.write(|tx| {
        if store::has_challenge(tx, kind.as_str(), &identifier_hmac)? {
            return Ok::<_, rusqlite::Error>(Outcome::AlreadyDisputed);
        }
        let challenge_id = store::add_challenge(
            tx,
            &NewChallenge {
                identifier_type: kind.as_str(),
                identifier_hmac: &identifier_hmac,
                claimant_name: pytext::strip(&claimant_name),
                contact_email: pytext::strip(&contact_email),
                proof_note: pytext::strip(&proof_note),
                created: &to_utc_text(now),
            },
        )?;
        let place = Context {
            identifier_type: Some(kind.as_str()),
            display: Some(activity::masked(kind, &normalised)),
            ..Context::at(now)
        };
        activity::record(tx, "dispute", &place, None, false, None)?;
        Ok(Outcome::Created(challenge_id))
    })?;
    match outcome {
        Outcome::Created(challenge_id) => Ok((
            StatusCode::CREATED,
            Json(ChallengeOut {
                challenge_id,
                status: "under_review",
            }),
        )),
        Outcome::AlreadyDisputed => Err(ApiError::new(
            StatusCode::TOO_MANY_REQUESTS,
            "a dispute is already under review for this number; only one is allowed",
        )),
    }
}
