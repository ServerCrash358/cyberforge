//! 400 rules: identifier type and value, pincode in the area table, `occurred_at` with an offset
//! and not in the future; `scam_type` and `channel` outside the lists are kept as "other".

use super::isotime::{self, ParseError};
use super::{areas, pytext};
use crate::shared;
use std::collections::HashSet;
use std::sync::LazyLock;
use time::{Duration, OffsetDateTime, UtcOffset};

/// Phones and servers disagree by a few seconds; a report "from the future" within this is fine.
pub const CLOCK_SKEW: Duration = Duration::minutes(2);

pub const OTHER_SCAM_TYPE: &str = "other";
pub const OTHER_CHANNEL: &str = "Something else";

/// A report field the API rejects with 400. The message never contains the value.
#[derive(Debug, PartialEq, Eq)]
pub struct InvalidReport(pub String);

static SCAM_TYPES: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| shared::SCAM_TYPES.iter().map(|t| t.id.as_str()).collect());
static CHANNELS: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| shared::CHANNELS.iter().map(|c| c.id.as_str()).collect());

/// An ISO 8601 time with an offset, in UTC.
///
/// # Errors
/// Not a time, or no offset.
pub fn parse_time(raw: &str, field: &str) -> Result<OffsetDateTime, InvalidReport> {
    match isotime::parse(pytext::strip(raw)) {
        Ok(when) => Ok(when.to_offset(UtcOffset::UTC)),
        Err(ParseError::NotIso) => Err(InvalidReport(format!(
            "invalid {field}: not an ISO 8601 time"
        ))),
        Err(ParseError::NoOffset) => Err(InvalidReport(format!(
            "invalid {field}: needs a timezone offset"
        ))),
    }
}

/// # Errors
/// Not a time with an offset, or later than `now` plus [`CLOCK_SKEW`].
pub fn occurred_at(raw: &str, now: OffsetDateTime) -> Result<OffsetDateTime, InvalidReport> {
    let when = parse_time(raw, "occurred_at")?;
    if when > now + CLOCK_SKEW {
        return Err(InvalidReport(
            "invalid occurred_at: in the future".to_owned(),
        ));
    }
    Ok(when)
}

/// The report's own `reported_at`, clamped between `occurred_at` and now; now when unusable.
///
/// The PRD's test tool posts many reports in seconds, so arrival time says nothing about when
/// people reported. A `reported_at` outside that range is clamped, never trusted as is.
pub fn reported_at(
    raw: Option<&str>,
    occurred: OffsetDateTime,
    now: OffsetDateTime,
) -> OffsetDateTime {
    match raw
        .filter(|r| !r.is_empty())
        .map(|r| parse_time(r, "reported_at"))
    {
        Some(Ok(when)) => when.max(occurred).min(now),
        _ => now,
    }
}

/// # Errors
/// A pincode that is not one of the known areas.
pub fn pincode(raw: &str) -> Result<String, InvalidReport> {
    let value = pytext::strip(raw);
    if !areas::is_known(value) {
        return Err(InvalidReport(
            "invalid pincode: not a known area".to_owned(),
        ));
    }
    Ok(value.to_owned())
}

/// Appendix C names as sent; anything else is accepted and stored as "other".
pub fn scam_type(raw: &str) -> String {
    let value = pytext::strip(raw);
    if SCAM_TYPES.contains(value) {
        value
    } else {
        OTHER_SCAM_TYPE
    }
    .to_owned()
}

pub fn channel(raw: &str) -> String {
    let value = pytext::strip(raw);
    if CHANNELS.contains(value) {
        value
    } else {
        OTHER_CHANNEL
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(raw: &str) -> OffsetDateTime {
        parse_time(raw, "x").unwrap()
    }

    #[test]
    fn occurred_at_rules() {
        let now = t("2026-11-01T12:00:00+05:30");
        assert!(occurred_at("2026-11-01T12:01:59+05:30", now).is_ok());
        assert_eq!(
            occurred_at("2026-11-01T12:03:00+05:30", now),
            Err(InvalidReport("invalid occurred_at: in the future".into()))
        );
        assert_eq!(
            occurred_at("2026-11-01T12:00:00", now),
            Err(InvalidReport(
                "invalid occurred_at: needs a timezone offset".into()
            ))
        );
        assert_eq!(
            occurred_at("yesterday", now),
            Err(InvalidReport(
                "invalid occurred_at: not an ISO 8601 time".into()
            ))
        );
    }

    #[test]
    fn reported_at_is_clamped() {
        let occurred = t("2026-11-01T10:00:00+05:30");
        let now = t("2026-11-01T12:00:00+05:30");
        assert_eq!(reported_at(None, occurred, now), now);
        assert_eq!(reported_at(Some(""), occurred, now), now);
        assert_eq!(reported_at(Some("junk"), occurred, now), now);
        assert_eq!(
            reported_at(Some("2026-11-01T09:00:00+05:30"), occurred, now),
            occurred
        );
        assert_eq!(
            reported_at(Some("2026-11-02T09:00:00+05:30"), occurred, now),
            now
        );
        let inside = t("2026-11-01T11:00:00+05:30");
        assert_eq!(
            reported_at(Some("2026-11-01T11:00:00+05:30"), occurred, now),
            inside
        );
    }

    #[test]
    fn pincode_scam_type_and_channel() {
        assert_eq!(pincode(" 560102 "), Ok("560102".to_owned()));
        assert!(pincode("123456").is_err());
        assert_eq!(scam_type("digital-arrest"), "digital-arrest");
        assert_eq!(scam_type("made-up"), "other");
        assert_eq!(channel("Phone call"), "Phone call");
        assert_eq!(channel("Pigeon"), "Something else");
    }
}
