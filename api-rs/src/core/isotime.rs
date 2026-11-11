//! ISO 8601 times as the Python API read and wrote them (`datetime.fromisoformat`, `isoformat`).
//!
//! Accepted: `YYYY-MM-DD`, then `T`, `t` or a space, then `HH:MM`, `HH:MM:SS` or
//! `HH:MM:SS.f` (1 to 9 digits, kept to the microsecond), then an offset `Z`, `±HH`, `±HHMM` or
//! `±HH:MM`. A time with no offset is refused by the report rules.

use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

/// India Standard Time, used when showing times, as in the PRD examples.
pub const DISPLAY_OFFSET: UtcOffset = match UtcOffset::from_hms(5, 30, 0) {
    Ok(offset) => offset,
    Err(_) => panic!("valid offset"),
};

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError {
    NotIso,
    NoOffset,
}

/// A parsed time with its offset.
///
/// # Errors
/// `NoOffset` for a valid time without an offset, `NotIso` for anything else that is not a time.
pub fn parse(raw: &str) -> Result<OffsetDateTime, ParseError> {
    let s = raw.as_bytes();
    if s.len() < 16 || !raw.is_ascii() {
        return Err(ParseError::NotIso);
    }
    let date = date(&s[..10]).ok_or(ParseError::NotIso)?;
    if !matches!(s[10], b'T' | b't' | b' ') {
        return Err(ParseError::NotIso);
    }
    let rest = &s[11..];
    let (clock, after) = clock(rest).ok_or(ParseError::NotIso)?;
    let when = PrimitiveDateTime::new(date, clock);
    if after.is_empty() {
        return Err(ParseError::NoOffset);
    }
    let offset = offset(after).ok_or(ParseError::NotIso)?;
    Ok(when.assume_offset(offset))
}

fn digits(s: &[u8]) -> Option<u32> {
    if s.is_empty() || !s.iter().all(u8::is_ascii_digit) {
        return None;
    }
    s.iter().try_fold(0u32, |n, d| {
        n.checked_mul(10)?.checked_add(u32::from(d - b'0'))
    })
}

fn date(s: &[u8]) -> Option<Date> {
    if s[4] != b'-' || s[7] != b'-' {
        return None;
    }
    let year = i32::try_from(digits(&s[..4])?).ok()?;
    let month = Month::try_from(u8::try_from(digits(&s[5..7])?).ok()?).ok()?;
    let day = u8::try_from(digits(&s[8..10])?).ok()?;
    Date::from_calendar_date(year, month, day).ok()
}

/// `HH:MM[:SS[.f]]` and what follows it.
fn clock(s: &[u8]) -> Option<(Time, &[u8])> {
    if s.len() < 5 || s[2] != b':' {
        return None;
    }
    let hour = u8::try_from(digits(&s[..2])?).ok()?;
    let minute = u8::try_from(digits(&s[3..5])?).ok()?;
    let mut rest = &s[5..];
    let mut second = 0u8;
    let mut micro = 0u32;
    if rest.first() == Some(&b':') {
        second = u8::try_from(digits(rest.get(1..3)?)?).ok()?;
        rest = &rest[3..];
        if rest.first() == Some(&b'.') {
            let len = rest[1..].iter().take_while(|c| c.is_ascii_digit()).count();
            if !(1..=9).contains(&len) {
                return None;
            }
            let mut frac: Vec<u8> = rest[1..=len].to_vec();
            frac.resize(6, b'0');
            micro = digits(&frac[..6])?;
            rest = &rest[1 + len..];
        }
    }
    Some((
        Time::from_hms_micro(hour, minute, second, micro).ok()?,
        rest,
    ))
}

fn offset(s: &[u8]) -> Option<UtcOffset> {
    if s == b"Z" || s == b"z" {
        return Some(UtcOffset::UTC);
    }
    let sign: i8 = match s.first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let body = &s[1..];
    let (h, m) = match body.len() {
        2 => (&body[..2], &b"00"[..]),
        4 => (&body[..2], &body[2..4]),
        5 if body[2] == b':' => (&body[..2], &body[3..5]),
        _ => return None,
    };
    let hours = i8::try_from(digits(h)?).ok()?;
    let minutes = i8::try_from(digits(m)?).ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    UtcOffset::from_hms(sign * hours, sign * minutes, 0).ok()
}

/// Python `isoformat()`: microseconds only when not zero, offset as `+HH:MM`.
pub fn format(when: OffsetDateTime) -> String {
    let micro = when.microsecond();
    let frac = if micro == 0 {
        String::new()
    } else {
        format!(".{micro:06}")
    };
    let offset = when.offset();
    let total = offset.whole_minutes();
    let sign = if total < 0 { '-' } else { '+' };
    let (oh, om) = (total.abs() / 60, total.abs() % 60);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{frac}{sign}{oh:02}:{om:02}",
        when.year(),
        u8::from(when.month()),
        when.day(),
        when.hour(),
        when.minute(),
        when.second(),
    )
}

/// How times are stored: UTC, Python `isoformat()`.
pub fn to_utc_text(when: OffsetDateTime) -> String {
    format(when.to_offset(UtcOffset::UTC))
}

/// A stored UTC time as shown in replies: India time, to the second.
///
/// # Panics
/// Only on text this module did not write.
pub fn to_display(utc_text: &str) -> String {
    let when = parse(utc_text).expect("stored times are valid");
    let shown = when.to_offset(DISPLAY_OFFSET);
    format(
        shown
            .replace_microsecond(0)
            .expect("zero is a valid microsecond"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(raw: &str) -> String {
        to_utc_text(parse(raw).unwrap())
    }

    #[test]
    fn reads_the_prd_format_and_its_variants() {
        assert_eq!(
            utc("2026-10-31T16:13:57+05:30"),
            "2026-10-31T10:43:57+00:00"
        );
        assert_eq!(utc("2026-10-31 16:13+05:30"), "2026-10-31T10:43:00+00:00");
        assert_eq!(utc("2026-10-31T10:43:57Z"), "2026-10-31T10:43:57+00:00");
        assert_eq!(utc("2026-10-31T06:43:57-0400"), "2026-10-31T10:43:57+00:00");
        assert_eq!(
            utc("2026-10-31T10:43:57.5+00"),
            "2026-10-31T10:43:57.500000+00:00"
        );
        assert_eq!(
            utc("2026-10-31T10:43:57.123456789+00:00"),
            "2026-10-31T10:43:57.123456+00:00"
        );
    }

    #[test]
    fn refuses_bad_times() {
        assert_eq!(parse("2026-10-31T10:43:57"), Err(ParseError::NoOffset));
        for bad in [
            "yesterday",
            "2026-13-01T00:00:00+00:00",
            "2026-02-30T00:00:00+00:00",
            "2026-10-31T24:00:00+00:00",
            "2026-10-31T10:43:57+24:00",
            "2026-10-31T10:43:57+5:30",
            "2026-10-31X10:43:57+00:00",
            "२०२६-10-31T10:43:57+00:00",
        ] {
            assert_eq!(parse(bad), Err(ParseError::NotIso), "{bad}");
        }
    }

    #[test]
    fn displays_in_india_time_to_the_second() {
        assert_eq!(
            to_display("2026-10-31T10:43:57.900000+00:00"),
            "2026-10-31T16:13:57+05:30"
        );
    }
}
