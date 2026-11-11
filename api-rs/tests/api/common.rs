//! A test app on an in-memory database, a fixed clock and a test licence key, and helpers.

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{Signer, SigningKey};
use http_body_util::BodyExt;
use scam_map_api::clock::FixedClock;
use scam_map_api::core::identity::Keyring;
use scam_map_api::db::Database;
use scam_map_api::license::verify::canonical_bytes;
use scam_map_api::state::AppState;
use serde_json::{Value, json};
use std::sync::Arc;
use time::OffsetDateTime;
use time::macros::datetime;
use tower::ServiceExt;

pub const NOW: OffsetDateTime = datetime!(2026-11-01 12:00 +05:30);

pub struct TestApp {
    pub router: Router,
    pub clock: Arc<FixedClock>,
}

/// The test licence key pair; never the real one.
pub fn signing_key() -> SigningKey {
    SigningKey::from_bytes(&[7; 32])
}

/// A licence file signed with the test key, as `scripts/license_sign.py` writes them.
pub fn licence(licensee: &str, expires_at: &str) -> Value {
    let signature = signing_key().sign(&canonical_bytes(licensee, expires_at));
    json!({
        "licensee": licensee,
        "expires_at": expires_at,
        "signature": STANDARD.encode(signature.to_bytes()),
    })
}

impl TestApp {
    /// An app with a year-long licence already loaded.
    pub async fn new() -> Self {
        let app = Self::unlicensed();
        let (status, _) = app
            .post("/license", &licence("Tests", "2027-11-01T12:00:00+05:30"))
            .await;
        assert_eq!(status, StatusCode::OK);
        app
    }

    /// An app with no licence: every route but /health and /license is locked.
    pub fn unlicensed() -> Self {
        let clock = Arc::new(FixedClock::new(NOW));
        let state = AppState::new(
            Database::in_memory().unwrap(),
            clock.clone(),
            Keyring::new("test-key-0123456789").unwrap(),
            Some(signing_key().verifying_key()),
        )
        .unwrap();
        Self {
            router: scam_map_api::app(state),
            clock,
        }
    }

    pub async fn send(&self, request: Request<Body>) -> (StatusCode, Value) {
        let response = self.router.clone().oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        };
        (status, body)
    }

    pub async fn get(&self, uri: &str) -> (StatusCode, Value) {
        self.send(Request::get(uri).body(Body::empty()).unwrap())
            .await
    }

    pub async fn post(&self, uri: &str, body: &Value) -> (StatusCode, Value) {
        let request = Request::post(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap();
        self.send(request).await
    }

    pub async fn lookup(&self, kind: &str, value: &str) -> (StatusCode, Value) {
        let query = serde_urlencoded::to_string([("type", kind), ("value", value)]).unwrap();
        self.get(&format!("/lookup?{query}")).await
    }
}

/// A valid PRD report from reporter `who` on connection `ip`, 10 minutes before NOW.
pub fn report(who: &str, ip: &str, value: &str, note: &str) -> Value {
    json!({
        "report_id": format!("rep_{who}_{ip}"),
        "occurred_at": "2026-11-01T11:50:00+05:30",
        "reported_at": "2026-11-01T11:55:00+05:30",
        "reporter_id": who,
        "ip_prefix": ip,
        "channel": "Phone call",
        "scam_type": "fake-customer-care",
        "identifier_type": "phone",
        "identifier_value": value,
        "pincode": "560102",
        "locality": "HSR Layout",
        "city": "Bengaluru",
        "note": note,
        "has_screenshot": false,
        "amount_lost_inr": 0
    })
}
