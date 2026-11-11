//! `GET /map`, `GET /alerts`, `POST /challenges`, `POST /admin/reset` and the `/app` helpers,
//! ported from the Python API's tests, on the fixed test clock.

use crate::common::{NOW, TestApp};
use axum::http::StatusCode;
use scam_map_api::core::areas;
use scam_map_api::core::isotime;
use serde_json::{Map, Value, json};
use std::sync::atomic::{AtomicUsize, Ordering};
use time::Duration;
use time::macros::offset;

const HSR: &str = "560102";
const BTM: &str = "560076";
const KORAMANGALA: &str = "560034";
const BANDRA: &str = "400050";

static IDS: AtomicUsize = AtomicUsize::new(1);

fn ist(hours_ago: f64) -> String {
    let when = NOW - Duration::seconds_f64(hours_ago * 3600.0);
    isotime::format(when.to_offset(offset!(+05:30)))
}

/// A counted report: a new person, connection and number, in their own words.
async fn report(app: &TestApp, pincode: &str, hours_ago: f64, scam_type: &str) {
    let n = IDS.fetch_add(1, Ordering::Relaxed);
    let body = json!({
        "report_id": format!("rep_{n}"),
        "occurred_at": ist(hours_ago),
        "reported_at": ist(hours_ago - 10.0 / 60.0),
        "reporter_id": format!("r_{n}"),
        "ip_prefix": format!("10.{}.{}", n % 250, n / 250),
        "channel": "Phone call",
        "scam_type": scam_type,
        "identifier_type": "phone",
        "identifier_value": format!("9{n:09}"),
        "pincode": pincode,
        "note": format!("Story {n}: the caller {} talked about thing {} for a while.", n * 7919, n * 31337),
    });
    assert_eq!(app.post("/reports", &body).await.0, StatusCode::CREATED);
}

async fn clusters(app: &TestApp, pincode: &str, extra: &str) -> Map<String, Value> {
    let (status, body) = app.get(&format!("/map?pincode={pincode}{extra}")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["clusters"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| (c["pincode"].as_str().unwrap().to_owned(), c.clone()))
        .collect()
}

async fn alerts(app: &TestApp, pincode: &str) -> Value {
    let (status, body) = app.get(&format!("/alerts?pincode={pincode}")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["alerts"].clone()
}

async fn steady(app: &TestApp, pincode: &str, per_day: usize, scam_type: &str) {
    for day in 0..7 {
        for k in 0..per_day {
            #[expect(clippy::cast_precision_loss, reason = "small test numbers")]
            let hours = (24 + 24 * day + 2 + 3 * k) as f64;
            report(app, pincode, hours, scam_type).await;
        }
    }
}

async fn today(app: &TestApp, pincode: &str, n: usize, scam_type: &str) {
    for k in 0..n {
        #[expect(clippy::cast_precision_loss, reason = "small test numbers")]
        report(app, pincode, (1 + 2 * k) as f64, scam_type).await;
    }
}

// ---------------------------------------------------------------- map

#[tokio::test]
async fn map_window_keeps_only_recent_reports() {
    let app = TestApp::new().await;
    for days in [6.0, 8.0, 30.0, 45.0] {
        report(&app, HSR, days * 24.0 + 2.0, "job-offer").await;
    }
    assert_eq!(clusters(&app, HSR, "&radius_km=0").await[HSR]["count"], 1);
    assert_eq!(
        clusters(&app, HSR, "&days=31&radius_km=0").await[HSR]["count"],
        3
    );
}

#[tokio::test]
async fn map_defaults_are_7_days_and_2_km() {
    let app = TestApp::new().await;
    report(&app, HSR, 24.0, "job-offer").await;
    let near: Vec<&str> = areas::all()
        .iter()
        .map(|a| a.pincode.as_str())
        .filter(|p| *p != HSR && areas::distance_km(HSR, p) <= 2.0)
        .collect();
    let far: Vec<&str> = areas::all()
        .iter()
        .map(|a| a.pincode.as_str())
        .filter(|p| areas::distance_km(HSR, p) > 2.0)
        .take(3)
        .collect();
    for p in near.iter().chain(&far) {
        report(&app, p, 24.0, "job-offer").await;
    }
    let mut shown: Vec<String> = clusters(&app, HSR, "").await.keys().cloned().collect();
    let mut expected: Vec<String> = near.iter().map(|p| (*p).to_owned()).collect();
    expected.push(HSR.to_owned());
    shown.sort();
    expected.sort();
    assert_eq!(shown, expected);
}

#[tokio::test]
async fn radius_edge_is_exact() {
    let app = TestApp::new().await;
    let d = areas::distance_km(HSR, KORAMANGALA);
    report(&app, HSR, 5.0, "job-offer").await;
    report(&app, KORAMANGALA, 5.0, "job-offer").await;
    let wider = format!("&radius_km={}", d + 0.05);
    let narrower = format!("&radius_km={}", d - 0.05);
    assert!(clusters(&app, HSR, &wider).await.contains_key(KORAMANGALA));
    assert!(
        !clusters(&app, HSR, &narrower)
            .await
            .contains_key(KORAMANGALA)
    );
}

#[tokio::test]
async fn clusters_sit_at_appendix_b_points_with_severity() {
    let app = TestApp::new().await;
    for _ in 0..12 {
        report(&app, HSR, 30.0, "digital-arrest").await;
    }
    report(&app, HSR, 30.0, "job-offer").await;
    assert_eq!(
        clusters(&app, HSR, "").await[HSR],
        json!({
            "pincode": HSR,
            "locality": "HSR Layout",
            "city": "Bengaluru",
            "count": 13,
            "top_scam_type": "digital-arrest",
            "severity": "CRIT",
            "lat": 12.9116,
            "lng": 77.6473
        })
    );
}

#[tokio::test]
async fn severity_bands() {
    for (n, severity) in [(1, "MED"), (4, "MED"), (5, "HIGH"), (10, "CRIT")] {
        let app = TestApp::new().await;
        for _ in 0..n {
            report(&app, BTM, 10.0, "job-offer").await;
        }
        assert_eq!(
            clusters(&app, BTM, "&radius_km=0").await[BTM]["severity"],
            severity
        );
    }
}

#[tokio::test]
async fn other_cities_never_appear() {
    let app = TestApp::new().await;
    report(&app, BANDRA, 10.0, "job-offer").await;
    assert!(
        !clusters(&app, HSR, "&radius_km=50")
            .await
            .contains_key(BANDRA)
    );
}

#[tokio::test]
async fn fakes_never_reach_the_map_or_alerts() {
    let app = TestApp::new().await;
    for n in 0..40_i64 {
        let body = json!({
            "report_id": format!("bomb_{n}"),
            "occurred_at": ist(3.0 + f64::from(u8::try_from(n % 10).unwrap()) / 60.0),
            "reported_at": ist(2.0 - f64::from(u8::try_from(n).unwrap()) / 60.0),
            "reporter_id": format!("friend_{n}"),
            "ip_prefix": "103.21.58",
            "channel": "Phone call",
            "scam_type": "fake-customer-care",
            "identifier_type": "phone",
            "identifier_value": "+91 80 4555 0123",
            "pincode": HSR,
            "note": "",
        });
        assert_eq!(app.post("/reports", &body).await.0, StatusCode::CREATED);
    }
    assert_eq!(clusters(&app, HSR, "").await, Map::new());
    assert_eq!(alerts(&app, HSR).await, json!([]));
}

#[tokio::test]
async fn bad_map_queries_are_400() {
    let app = TestApp::new().await;
    for (query, message) in [
        ("", "missing pincode"),
        ("?pincode=999999", "invalid pincode: not a known area"),
        ("?pincode=56010", "invalid pincode"),
        ("?pincode=560102&days=0", "invalid days"),
        ("?pincode=560102&days=week", "invalid days"),
        ("?pincode=560102&radius_km=-1", "invalid radius_km"),
    ] {
        assert_eq!(
            app.get(&format!("/map{query}")).await,
            (StatusCode::BAD_REQUEST, json!({"error": message})),
            "{query}"
        );
    }
}

// ---------------------------------------------------------------- alerts

#[tokio::test]
async fn spike_when_double_the_baseline_and_at_least_five() {
    let app = TestApp::new().await;
    steady(&app, HSR, 2, "job-offer").await;
    today(&app, HSR, 10, "job-offer").await;
    assert_eq!(
        alerts(&app, HSR).await,
        json!([{"type": "SPIKE", "pincode": HSR, "window": "24h", "count": 10, "baseline_per_day": 2.0}])
    );
}

#[tokio::test]
async fn exactly_double_with_six_is_a_spike_but_four_is_quiet() {
    let app = TestApp::new().await;
    steady(&app, HSR, 3, "job-offer").await;
    today(&app, HSR, 6, "job-offer").await;
    assert_eq!(alerts(&app, HSR).await[0]["type"], "SPIKE");

    let quiet = TestApp::new().await;
    steady(&quiet, HSR, 2, "job-offer").await;
    today(&quiet, HSR, 4, "job-offer").await;
    assert_eq!(alerts(&quiet, HSR).await, json!([]));
}

#[tokio::test]
async fn steady_area_is_quiet() {
    let app = TestApp::new().await;
    steady(&app, HSR, 2, "job-offer").await;
    today(&app, HSR, 2, "job-offer").await;
    assert_eq!(alerts(&app, HSR).await, json!([]));
}

#[tokio::test]
async fn new_type_in_the_last_day() {
    let app = TestApp::new().await;
    steady(&app, HSR, 1, "job-offer").await;
    today(&app, HSR, 1, "sextortion").await;
    assert_eq!(
        alerts(&app, HSR).await,
        json!([{"type": "NEW_TYPE", "pincode": HSR, "scam_type": "sextortion", "window": "24h"}])
    );
}

#[tokio::test]
async fn a_type_seen_29_days_ago_is_not_new_but_31_is() {
    let app = TestApp::new().await;
    report(&app, HSR, 29.0 * 24.0, "sextortion").await;
    today(&app, HSR, 1, "sextortion").await;
    assert_eq!(alerts(&app, HSR).await, json!([]));

    let again = TestApp::new().await;
    report(&again, HSR, 31.5 * 24.0, "sextortion").await;
    today(&again, HSR, 1, "sextortion").await;
    assert_eq!(alerts(&again, HSR).await[0]["type"], "NEW_TYPE");
}

#[tokio::test]
async fn unknown_scam_types_never_raise_new_type() {
    let app = TestApp::new().await;
    steady(&app, HSR, 1, "job-offer").await;
    today(&app, HSR, 1, "crypto-doubler").await;
    assert_eq!(alerts(&app, HSR).await, json!([]));
}

#[tokio::test]
async fn bad_alert_queries_are_400() {
    let app = TestApp::new().await;
    assert_eq!(
        app.get("/alerts?pincode=999999").await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        app.get("/alerts").await,
        (StatusCode::BAD_REQUEST, json!({"error": "missing pincode"}))
    );
}

// ---------------------------------------------------------------- challenges

fn challenge(value: &str) -> Value {
    json!({
        "identifier_type": "phone",
        "identifier_value": value,
        "claimant_name": "Fresh Bakes",
        "contact_email": "owner@example.com",
        "proof_note": "GST certificate and shop licence attached.",
    })
}

const BAKERY: &str = "+91 80 4555 0123";

async fn known_scam(app: &TestApp) {
    for n in 0..5_u8 {
        let body = json!({
            "report_id": format!("real_{n}"),
            "occurred_at": ist(f64::from(n + 1) * 24.0),
            "reporter_id": format!("real_{n}"),
            "ip_prefix": format!("49.{n}.1"),
            "channel": "Phone call",
            "scam_type": "fake-customer-care",
            "identifier_type": "phone",
            "identifier_value": "08045550123",
            "pincode": HSR,
            "note": format!("Person {n} wrote this in their own way, number {} and day {}.", u32::from(n) * 7919, n * 13),
        });
        assert_eq!(app.post("/reports", &body).await.0, StatusCode::CREATED);
    }
}

async fn bakery(app: &TestApp) -> Value {
    app.lookup("phone", "+918045550123").await.1
}

#[tokio::test]
async fn a_challenge_marks_the_number_and_caps_the_verdict() {
    let app = TestApp::new().await;
    known_scam(&app).await;
    assert_eq!(bakery(&app).await["verdict"], "KNOWN_SCAM");
    let (status, body) = app.post("/challenges", &challenge(BAKERY)).await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["status"], "under_review");
    assert!(body["challenge_id"].as_str().unwrap().starts_with("chl_"));
    let after = bakery(&app).await;
    assert_eq!(
        (after["challenged"].clone(), after["verdict"].clone()),
        (json!(true), json!("SUSPICIOUS"))
    );
}

#[tokio::test]
async fn a_challenge_before_any_report_still_holds() {
    let app = TestApp::new().await;
    app.post("/challenges", &challenge(BAKERY)).await;
    known_scam(&app).await;
    assert_eq!(bakery(&app).await["verdict"], "SUSPICIOUS");
}

#[tokio::test]
async fn claimant_contact_is_never_returned() {
    let app = TestApp::new().await;
    known_scam(&app).await;
    let replies = [
        app.post("/challenges", &challenge(BAKERY))
            .await
            .1
            .to_string(),
        app.post("/challenges", &challenge(BAKERY))
            .await
            .1
            .to_string(),
        bakery(&app).await.to_string(),
        app.get("/app/activity").await.1.to_string(),
    ];
    for reply in replies {
        assert!(
            !reply.contains("owner@example.com") && !reply.contains("Fresh Bakes"),
            "{reply}"
        );
    }
}

#[tokio::test]
async fn one_dispute_per_number_in_any_spelling() {
    let app = TestApp::new().await;
    assert_eq!(
        app.post("/challenges", &challenge(BAKERY)).await.0,
        StatusCode::CREATED
    );
    for spelling in ["+91 80 4555 0123", "080-45550123", "9180 45550123"] {
        let (status, body) = app.post("/challenges", &challenge(spelling)).await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{spelling}");
        assert!(body["error"].as_str().unwrap().contains("already"));
    }
    assert_eq!(
        app.post("/challenges", &challenge("+91 80 4555 0999"))
            .await
            .0,
        StatusCode::CREATED
    );
    let events = app.get("/app/activity").await.1["events"].clone();
    let disputes = events
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["kind"] == "dispute")
        .count();
    assert_eq!(disputes, 2);
}

#[tokio::test]
async fn bad_challenges_are_400() {
    let app = TestApp::new().await;
    assert_eq!(
        app.post("/challenges", &challenge("12345")).await,
        (
            StatusCode::BAD_REQUEST,
            json!({"error": "invalid identifier_value for type phone"})
        )
    );
    let mut nameless = challenge(BAKERY);
    nameless.as_object_mut().unwrap().remove("claimant_name");
    assert_eq!(
        app.post("/challenges", &nameless).await,
        (
            StatusCode::BAD_REQUEST,
            json!({"error": "missing claimant_name"})
        )
    );
    let mut long_note = challenge(BAKERY);
    long_note["proof_note"] = json!("x".repeat(2001));
    assert_eq!(
        app.post("/challenges", &long_note).await,
        (
            StatusCode::BAD_REQUEST,
            json!({"error": "invalid proof_note"})
        )
    );
}

#[tokio::test]
async fn a_verified_number_can_be_disputed_once_and_stays_verified() {
    let app = TestApp::new().await;
    assert_eq!(
        app.post("/challenges", &challenge("1930")).await.0,
        StatusCode::CREATED
    );
    assert_eq!(
        app.post("/challenges", &challenge("1930")).await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    assert_eq!(
        app.lookup("phone", "1930").await.1["verdict"],
        "VERIFIED_BUSINESS"
    );
}

// ---------------------------------------------------------------- reset and /app helpers

#[tokio::test]
async fn reset_empties_everything_but_keeps_the_licence() {
    let app = TestApp::new().await;
    known_scam(&app).await;
    app.post("/challenges", &challenge(BAKERY)).await;
    let licence = app.get("/license").await;
    assert_eq!(
        app.post("/admin/reset", &json!({})).await,
        (StatusCode::OK, json!({"ok": true}))
    );
    let after = bakery(&app).await;
    assert_eq!(
        (after["verdict"].clone(), after["challenged"].clone()),
        (json!("NO_REPORTS"), json!(false))
    );
    assert_eq!(
        app.get("/app/activity").await.1,
        json!({"events": [], "alerts": []})
    );
    assert_eq!(app.get("/license").await, licence);
    assert_eq!(
        app.post("/challenges", &challenge(BAKERY)).await.0,
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn app_helpers() {
    let app = TestApp::new().await;
    let (status, body) = app.get("/").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["routes"].as_array().unwrap().len(), 9);

    let (_, body) = app.get("/app/areas").await;
    let first = &body["areas"][0];
    assert_eq!(first["pincode"], HSR);
    assert_eq!(first["source"], "prd");
    assert_eq!(body["areas"].as_array().unwrap().len(), areas::all().len());

    steady(&app, HSR, 2, "job-offer").await;
    today(&app, HSR, 10, "job-offer").await;
    let (_, feed) = app.get("/app/activity").await;
    assert_eq!(feed["alerts"][0]["type"], "SPIKE");
    assert_eq!(feed["alerts"][0]["locality"], "HSR Layout");
    let first = &feed["events"][0];
    assert!(
        first["display"].as_str().unwrap().contains('X'),
        "always masked: {first}"
    );
    assert!(
        feed["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["kind"] == "alert")
    );
}
