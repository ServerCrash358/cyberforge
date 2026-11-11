//! `POST /reports` and `GET /lookup` through the real router: statuses, replies, trust rules.

use crate::common::{TestApp, report};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;

const NUMBER: &str = "+91 98765 43210";
const NOTES: [&str; 3] = [
    "Caller said my bank KYC expired and asked for the OTP",
    "SMS with a link to pay an electricity bill or lose power tonight",
    "Someone on video call said they were police and I was under arrest",
];

#[tokio::test]
async fn a_valid_report_is_201() {
    let app = TestApp::new().await;
    let (status, body) = app
        .post("/reports", &report("r1", "1.2.3", NUMBER, NOTES[0]))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    assert_eq!(body["report_id"], "rep_r1_1.2.3");
    assert_eq!(body["merged"], false);
    assert!(body["case_id"].as_str().unwrap().starts_with("case_"));

    let (_, second) = app
        .post("/reports", &report("r2", "4.5.6", NUMBER, NOTES[1]))
        .await;
    assert_eq!(second["merged"], true);
    assert_eq!(second["case_id"], body["case_id"]);
}

#[tokio::test]
async fn bad_bodies_are_400_and_never_echo_values() {
    let app = TestApp::new().await;
    let mut missing = report("r1", "1.2.3", NUMBER, "");
    missing.as_object_mut().unwrap().remove("report_id");
    missing.as_object_mut().unwrap().remove("pincode");
    assert_eq!(
        app.post("/reports", &missing).await,
        (
            StatusCode::BAD_REQUEST,
            json!({"error": "missing report_id; missing pincode"})
        )
    );

    let mut bad_type = report("r1", "1.2.3", NUMBER, "");
    bad_type["identifier_type"] = json!("email");
    assert_eq!(
        app.post("/reports", &bad_type).await,
        (
            StatusCode::BAD_REQUEST,
            json!({"error": "invalid identifier_type"})
        )
    );

    let junk = report("r1", "1.2.3", "12345", "");
    assert_eq!(
        app.post("/reports", &junk).await,
        (
            StatusCode::BAD_REQUEST,
            json!({"error": "invalid identifier_value for type phone"})
        )
    );

    let mut nowhere = report("r1", "1.2.3", NUMBER, "");
    nowhere["pincode"] = json!("999999");
    assert_eq!(
        app.post("/reports", &nowhere).await,
        (
            StatusCode::BAD_REQUEST,
            json!({"error": "invalid pincode: not a known area"})
        )
    );

    let mut future = report("r1", "1.2.3", NUMBER, "");
    future["occurred_at"] = json!("2026-11-02T12:00:00+05:30");
    assert_eq!(
        app.post("/reports", &future).await,
        (
            StatusCode::BAD_REQUEST,
            json!({"error": "invalid occurred_at: in the future"})
        )
    );

    let not_json = Request::post("/reports")
        .header("content-type", "application/json")
        .body(Body::from("{not json"))
        .unwrap();
    assert_eq!(
        app.send(not_json).await,
        (
            StatusCode::BAD_REQUEST,
            json!({"error": "body is not valid JSON"})
        )
    );
}

#[tokio::test]
async fn more_than_three_an_hour_is_429() {
    let app = TestApp::new().await;
    for i in 0..3 {
        let body = report("spammer", &format!("ip{i}"), &format!("98765432{i}0"), "");
        assert_eq!(app.post("/reports", &body).await.0, StatusCode::CREATED);
    }
    let (status, body) = app
        .post("/reports", &report("spammer", "ip9", "9876543290", ""))
        .await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        body,
        json!({"error": "too many reports from this reporter; try again later"})
    );
}

#[tokio::test]
async fn the_rate_limit_lifts_after_an_hour() {
    let app = TestApp::new().await;
    let at = |minutes: i64| {
        let when = crate::common::NOW + time::Duration::minutes(minutes);
        scam_map_api::core::isotime::format(when.to_offset(time::macros::offset!(+05:30)))
    };
    for i in 0..4_i64 {
        let mut body = report("busy", &format!("ip{i}"), &format!("98765432{i}0"), "");
        body["reported_at"] = json!(at(0));
        body["occurred_at"] = json!(at(-5));
        let expected = if i < 3 {
            StatusCode::CREATED
        } else {
            StatusCode::TOO_MANY_REQUESTS
        };
        assert_eq!(app.post("/reports", &body).await.0, expected);
    }
    app.clock.advance(time::Duration::minutes(61));
    let mut later = report("busy", "ip9", "9876543290", "");
    later["occurred_at"] = json!(at(60));
    later["reported_at"] = json!(at(61));
    assert_eq!(app.post("/reports", &later).await.0, StatusCode::CREATED);
}

#[tokio::test]
async fn unreported_is_no_reports_and_masked() {
    let app = TestApp::new().await;
    let (status, body) = app.lookup("phone", NUMBER).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({
            "verdict": "NO_REPORTS",
            "report_count": 0,
            "distinct_reporters": 0,
            "first_seen": null,
            "last_seen": null,
            "scam_type": null,
            "display_value": "98XXXXX210",
            "evidence": [],
            "challenged": false
        })
    );
}

#[tokio::test]
async fn three_people_make_a_known_scam_and_unmask_it() {
    let app = TestApp::new().await;
    for (i, note) in NOTES.iter().enumerate() {
        app.post(
            "/reports",
            &report(&format!("p{i}"), &format!("10.0.{i}"), NUMBER, note),
        )
        .await;
        let (_, body) = app.lookup("phone", "09876543210").await;
        let expected = if i < 2 { "SUSPICIOUS" } else { "KNOWN_SCAM" };
        assert_eq!(body["verdict"], expected);
    }
    let (_, body) = app.lookup("phone", "919876543210").await;
    assert_eq!(body["distinct_reporters"], 3);
    assert_eq!(body["display_value"], "+919876543210");
    assert_eq!(body["first_seen"], "2026-11-01T11:50:00+05:30");
    assert_eq!(body["scam_type"], "fake-customer-care");
    assert_eq!(body["evidence"].as_array().unwrap().len(), 3);
    assert_eq!(
        body["evidence"][0],
        json!({
            "occurred_at": "2026-11-01T11:50:00+05:30",
            "channel": "Phone call",
            "scam_type": "fake-customer-care",
            "pincode": "560102"
        })
    );
}

#[tokio::test]
async fn one_person_many_times_stays_suspicious() {
    let app = TestApp::new().await;
    for (i, note) in NOTES.iter().enumerate() {
        app.post(
            "/reports",
            &report("same", &format!("10.0.{i}"), NUMBER, note),
        )
        .await;
    }
    let (_, body) = app.lookup("phone", NUMBER).await;
    assert_eq!(body["verdict"], "SUSPICIOUS");
    assert_eq!(body["report_count"], 3);
    assert_eq!(body["distinct_reporters"], 1);
    assert_eq!(body["display_value"], "98XXXXX210");
}

#[tokio::test]
async fn verified_numbers_show_in_full_with_no_evidence() {
    let app = TestApp::new().await;
    app.post("/reports", &report("r1", "1.2.3", "1930", NOTES[0]))
        .await;
    let (_, body) = app.lookup("phone", "1930").await;
    assert_eq!(body["verdict"], "VERIFIED_BUSINESS");
    assert_eq!(body["display_value"], "1930");
    assert_eq!(body["evidence"], json!([]));
}

#[tokio::test]
async fn bad_lookups_are_400() {
    let app = TestApp::new().await;
    assert_eq!(
        app.get("/lookup").await,
        (
            StatusCode::BAD_REQUEST,
            json!({"error": "missing type; missing value"})
        )
    );
    assert_eq!(
        app.lookup("email", "x").await,
        (StatusCode::BAD_REQUEST, json!({"error": "invalid type"}))
    );
    assert_eq!(
        app.lookup("phone", "12345").await,
        (
            StatusCode::BAD_REQUEST,
            json!({"error": "invalid value for type phone"})
        )
    );
}
