//! Injectable clock: the single source of "now" for windows, rate limits and licence expiry.
//! Tests swap it for a [`FixedClock`] to travel in time.

use std::sync::{Mutex, PoisonError};
use time::{Duration, OffsetDateTime, UtcOffset};

pub trait Clock: Send + Sync {
    /// Current time in UTC.
    fn now(&self) -> OffsetDateTime;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

/// A clock that only moves when a test moves it.
pub struct FixedClock(Mutex<OffsetDateTime>);

impl FixedClock {
    pub fn new(start: OffsetDateTime) -> Self {
        Self(Mutex::new(start.to_offset(UtcOffset::UTC)))
    }

    pub fn set(&self, when: OffsetDateTime) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = when.to_offset(UtcOffset::UTC);
    }

    pub fn advance(&self, by: Duration) {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) += by;
    }
}

impl Clock for FixedClock {
    fn now(&self) -> OffsetDateTime {
        *self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn fixed_clock_moves_only_when_told() {
        let clock = FixedClock::new(datetime!(2026-11-01 12:00 +05:30));
        assert_eq!(clock.now(), datetime!(2026-11-01 06:30 UTC));
        clock.advance(Duration::hours(1));
        assert_eq!(clock.now(), datetime!(2026-11-01 07:30 UTC));
        clock.set(datetime!(2026-12-01 00:00 UTC));
        assert_eq!(clock.now(), datetime!(2026-12-01 00:00 UTC));
    }
}
