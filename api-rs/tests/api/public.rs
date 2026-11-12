//! Settings for a public server: the admin token on reset, and client addresses behind Caddy.

use crate::common::{TestApp, report};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use scam_map_api::state::Options;
use serde_json::{Value, json};

async fn reset_with(app: &TestApp, token: Option<&str>) -> (StatusCode, Value) {
    let mut request = Request::post("/admin/reset");
    if let Some(t) = token {
        request = request.header("x-admin-token", t);
    }
    app.send(request.body(Body::empty()).unwrap()).await
}

#[tokio::test]
async fn with_a_token_reset_needs_it() {
    let app = TestApp::with_options(Options {
        admin_token: Some("s3cret-token".into()),
        trust_proxy: false,
    })
    .await;
    let refused = (
        StatusCode::UNAUTHORIZED,
        json!({"error": "missing or wrong admin token"}),
    );
    assert_eq!(reset_with(&app, None).await, refused);
    assert_eq!(reset_with(&app, Some("wrong")).await, refused);
    assert_eq!(
        reset_with(&app, Some("s3cret-token")).await,
        (StatusCode::OK, json!({"ok": true}))
    );
}

#[tokio::test]
async fn without_a_token_reset_stays_open_as_the_prd_requires() {
    let app = TestApp::new().await;
    assert_eq!(reset_with(&app, None).await.0, StatusCode::OK);
}

/// A website report: no `ip_prefix`, so the server works out the connection itself.
async fn web_report(app: &TestApp, who: &str, forwarded: &str) {
    let mut body = report(who, "", "+91 98765 43210", "");
    body.as_object_mut().unwrap().remove("ip_prefix");
    let request = Request::post("/reports")
        .header("content-type", "application/json")
        .header("x-forwarded-for", forwarded)
        .body(Body::from(body.to_string()))
        .unwrap();
    assert_eq!(app.send(request).await.0, StatusCode::CREATED);
}

async fn people(app: &TestApp) -> Value {
    app.lookup("phone", "+91 98765 43210").await.1["distinct_reporters"].clone()
}

#[tokio::test]
async fn behind_caddy_the_last_forwarded_address_is_the_connection() {
    let app = TestApp::with_options(Options {
        admin_token: None,
        trust_proxy: true,
    })
    .await;
    web_report(&app, "a", "203.0.113.7").await;
    web_report(&app, "b", "198.51.100.9").await;
    assert_eq!(people(&app).await, 2, "two different connections");

    // A client cannot pose as another network: only the entry Caddy added counts.
    web_report(&app, "c", "192.0.2.1, 203.0.113.99").await;
    assert_eq!(
        people(&app).await,
        2,
        "same /24 as the first reporter: one person"
    );
}
