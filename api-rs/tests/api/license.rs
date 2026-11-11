//! The licence through the real router: the PRD's four-step demo, the gate, and bad files.

use crate::common::{NOW, TestApp, licence, report};
use axum::http::StatusCode;
use serde_json::json;
use time::Duration;

const LOCKED: (StatusCode, &str) = (StatusCode::FORBIDDEN, "LICENSE_EXPIRED");

async fn lookup_status(app: &TestApp) -> (StatusCode, serde_json::Value) {
    app.lookup("phone", "+91 98765 43210").await
}

#[tokio::test]
async fn without_a_licence_everything_but_health_and_license_is_locked() {
    let app = TestApp::unlicensed();
    let locked = (LOCKED.0, json!({"error": LOCKED.1}));
    assert_eq!(lookup_status(&app).await, locked);
    assert_eq!(
        app.post("/reports", &report("r1", "1.2.3", "9876543210", ""))
            .await,
        locked
    );
    assert_eq!(app.get("/nope").await, locked);
    assert_eq!(
        app.get("/health").await,
        (StatusCode::OK, json!({"ok": true}))
    );
    assert_eq!(
        app.get("/license").await,
        (
            StatusCode::OK,
            json!({"licensee": null, "expires_at": null, "valid": false})
        )
    );
}

/// PRD: 1 the app works, 2 load a licence that expires in 5 minutes, 3 after it expires the API
/// is locked, 4 load a long licence and everything works again with the old data.
#[tokio::test]
async fn the_four_step_demo_without_a_restart() {
    let app = TestApp::new().await;
    app.post(
        "/reports",
        &report("r1", "1.2.3", "9876543210", "Caller asked for my OTP twice"),
    )
    .await;
    let before = lookup_status(&app).await;
    assert_eq!(before.0, StatusCode::OK);
    assert_eq!(before.1["report_count"], 1);

    let short = licence("Demo Bank", "2026-11-01T12:05:00+05:30");
    assert_eq!(
        app.post("/license", &short).await,
        (
            StatusCode::OK,
            json!({"licensee": "Demo Bank", "expires_at": "2026-11-01T12:05:00+05:30", "valid": true})
        )
    );
    assert_eq!(lookup_status(&app).await, before);

    app.clock.advance(Duration::minutes(5));
    assert_eq!(
        lookup_status(&app).await,
        (LOCKED.0, json!({"error": LOCKED.1}))
    );
    assert_eq!(app.get("/health").await.0, StatusCode::OK);
    assert_eq!(app.get("/license").await.1["valid"], false);

    let long = licence("Demo Bank", "2027-11-01T12:00:00+05:30");
    assert_eq!(app.post("/license", &long).await.1["valid"], true);
    assert_eq!(
        lookup_status(&app).await,
        before,
        "the old data is still there"
    );
}

#[tokio::test]
async fn setting_the_clock_back_does_not_unlock() {
    let app = TestApp::new().await;
    app.post(
        "/license",
        &licence("Demo Bank", "2026-11-01T12:05:00+05:30"),
    )
    .await;
    app.clock.advance(Duration::minutes(10));
    assert_eq!(lookup_status(&app).await.0, StatusCode::FORBIDDEN);
    app.clock.set(NOW);
    assert_eq!(lookup_status(&app).await.0, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn bad_licence_files_are_400_and_change_nothing() {
    let app = TestApp::new().await;
    let current = app.get("/license").await;

    let mut edited = licence("Demo Bank", "2030-01-01T00:00:00+05:30");
    edited["expires_at"] = json!("2031-01-01T00:00:00+05:30");
    let expired = licence("Demo Bank", "2026-01-01T00:00:00+05:30");
    let mut missing = licence("Demo Bank", "2030-01-01T00:00:00+05:30");
    missing.as_object_mut().unwrap().remove("signature");
    for (doc, message) in [
        (edited, "signature does not match"),
        (expired, "licence has already expired"),
        (missing, "missing signature"),
    ] {
        assert_eq!(
            app.post("/license", &doc).await,
            (StatusCode::BAD_REQUEST, json!({"error": message}))
        );
    }
    assert_eq!(app.get("/license").await, current);
}
