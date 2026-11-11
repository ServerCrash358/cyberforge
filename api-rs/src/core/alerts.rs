//! SPIKE and `NEW_TYPE` rules over counted reports in one area, windows from `occurred_at`.
//!
//! PRD Appendix A, route 5:
//!   SPIKE    reports in the last 24 hours are at least double the daily average of the 7 days
//!            before, and at least 5.
//!   `NEW_TYPE` a scam type reported in the last 24 hours that was not seen in that area in the
//!            30 days before.
//! Only counted reports are passed in, so fakes the trust rules ignored never set off an alert.

use serde::Serialize;
use std::collections::BTreeSet;
use time::{Duration, OffsetDateTime};

pub const WINDOW: Duration = Duration::hours(24);
pub const BASELINE_DAYS: i64 = 7;
pub const NEW_TYPE_LOOKBACK: Duration = Duration::days(30);
pub const SPIKE_MIN: usize = 5;
pub const SPIKE_FACTOR: f64 = 2.0;
/// "other" collects every unrecognised type, so it is not a kind of scam people can act on.
pub const UNKNOWN_TYPE: &str = "other";

/// How far back alerts need to read.
pub const HISTORY: Duration = Duration::hours(24 + 30 * 24);

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum Alert {
    #[serde(rename = "SPIKE")]
    Spike {
        pincode: String,
        window: &'static str,
        count: usize,
        baseline_per_day: f64,
    },
    #[serde(rename = "NEW_TYPE")]
    NewType {
        pincode: String,
        scam_type: String,
        window: &'static str,
    },
}

fn round2(x: f64) -> f64 {
    format!("{x:.2}").parse().expect("a formatted float parses")
}

/// `events`: (`occurred_at`, `scam_type`) of counted reports in the area.
pub fn alerts(
    pincode: &str,
    events: &[(OffsetDateTime, String)],
    now: OffsetDateTime,
) -> Vec<Alert> {
    let recent_start = now - WINDOW;
    let baseline_start = recent_start - Duration::days(BASELINE_DAYS);
    let lookback_start = recent_start - NEW_TYPE_LOOKBACK;

    let recent: Vec<&(OffsetDateTime, String)> = events
        .iter()
        .filter(|(t, _)| recent_start < *t && *t <= now)
        .collect();
    let baseline = events
        .iter()
        .filter(|(t, _)| baseline_start < *t && *t <= recent_start)
        .count();
    #[expect(clippy::cast_precision_loss, reason = "counts are small")]
    let per_day = baseline as f64 / BASELINE_DAYS as f64;

    let mut found = Vec::new();
    #[expect(clippy::cast_precision_loss, reason = "counts are small")]
    let spiking = recent.len() >= SPIKE_MIN && recent.len() as f64 >= SPIKE_FACTOR * per_day;
    if spiking {
        found.push(Alert::Spike {
            pincode: pincode.to_owned(),
            window: "24h",
            count: recent.len(),
            baseline_per_day: round2(per_day),
        });
    }

    let seen_before: BTreeSet<&str> = events
        .iter()
        .filter(|(t, _)| lookback_start < *t && *t <= recent_start)
        .map(|(_, kind)| kind.as_str())
        .collect();
    let new_kinds: BTreeSet<&str> = recent
        .iter()
        .map(|(_, kind)| kind.as_str())
        .filter(|kind| !seen_before.contains(kind) && *kind != UNKNOWN_TYPE)
        .collect();
    for kind in new_kinds {
        found.push(Alert::NewType {
            pincode: pincode.to_owned(),
            scam_type: kind.to_owned(),
            window: "24h",
        });
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    const NOW: OffsetDateTime = datetime!(2026-11-01 12:00 UTC);

    fn ago(hours: i64, kind: &str) -> (OffsetDateTime, String) {
        (NOW - Duration::hours(hours), kind.to_owned())
    }

    #[test]
    fn a_spike_needs_five_and_double_the_baseline() {
        let mut events: Vec<_> = (0..5).map(|h| ago(h, "kyc")).collect();
        events.extend((0..7).map(|d| ago(30 + d * 24, "kyc")));
        let found = alerts("560102", &events, NOW);
        assert_eq!(
            found[0],
            Alert::Spike {
                pincode: "560102".into(),
                window: "24h",
                count: 5,
                baseline_per_day: 1.0
            }
        );
        assert_eq!(found.len(), 1, "kyc was seen before, so no NEW_TYPE");
        assert_eq!(alerts("560102", &events[1..], NOW), Vec::<Alert>::new());
    }

    #[test]
    fn new_types_are_sorted_and_skip_other() {
        let events = vec![
            ago(1, "parcel"),
            ago(2, "loan-app"),
            ago(3, "other"),
            ago(48, "kyc"),
        ];
        let kinds: Vec<_> = alerts("560102", &events, NOW)
            .into_iter()
            .map(|a| match a {
                Alert::NewType { scam_type, .. } => scam_type,
                Alert::Spike { .. } => unreachable!(),
            })
            .collect();
        assert_eq!(kinds, ["loan-app", "parcel"]);
    }

    #[test]
    fn json_matches_the_prd() {
        let a = Alert::Spike {
            pincode: "1".into(),
            window: "24h",
            count: 20,
            baseline_per_day: 5.43,
        };
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            r#"{"type":"SPIKE","pincode":"1","window":"24h","count":20,"baseline_per_day":5.43}"#
        );
        assert_eq!(round2(38.0 / 7.0), 5.43);
    }
}
