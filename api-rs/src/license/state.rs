//! The verified licence in memory, persisted to `license_state`, plus the clock high-water mark.
//!
//! The licence is verified once when it arrives (or at start-up), so the per-request check is
//! only a time comparison. Expired means locked: nothing here ever deletes data.

use super::verify::{InvalidLicence, Licence, LicenceDoc, verify};
use crate::core::isotime;
use crate::db::Database;
use ed25519_dalek::VerifyingKey;
use rusqlite::params;
use std::sync::{Mutex, PoisonError};
use time::{Duration, OffsetDateTime};

/// A clock this far behind the latest time seen counts as set back (PRD "Bonus thinking").
pub const ROLLBACK_TOLERANCE: Duration = Duration::minutes(1);
/// The high-water mark is written to disk at most this often; a restart loses at most this much.
pub const PERSIST_EVERY: Duration = Duration::minutes(1);

#[derive(Default)]
struct Held {
    licence: Option<Licence>,
    high_water: Option<OffsetDateTime>,
    persisted: Option<OffsetDateTime>,
}

/// Why a new licence was not installed.
#[derive(Debug)]
pub enum InstallError {
    /// The file is not a valid, current licence; the API replies 400.
    Invalid(InvalidLicence),
    /// It could not be saved; the current licence stays in force.
    Database(rusqlite::Error),
}

impl From<InvalidLicence> for InstallError {
    fn from(e: InvalidLicence) -> Self {
        Self::Invalid(e)
    }
}

impl From<rusqlite::Error> for InstallError {
    fn from(e: rusqlite::Error) -> Self {
        Self::Database(e)
    }
}

pub struct LicenceState {
    key: Option<VerifyingKey>,
    held: Mutex<Held>,
}

impl LicenceState {
    /// Reads the saved licence and high-water mark. A saved licence that no longer verifies
    /// (another key, an edited row) is dropped, which locks the API until a new one is loaded.
    ///
    /// # Errors
    /// Any database error.
    pub fn load(db: &Database, key: Option<VerifyingKey>) -> rusqlite::Result<Self> {
        let (json, high_water): (Option<String>, Option<String>) = db.read(|c| {
            c.query_row(
                "SELECT licence_json, high_water FROM license_state WHERE id = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
        })?;
        let high_water = high_water.and_then(|t| isotime::parse(&t).ok());
        let licence = json
            .and_then(|j| serde_json::from_str::<LicenceDoc>(&j).ok())
            .and_then(|doc| verify(&doc, key.as_ref()).ok());
        let held = Held {
            licence,
            high_water,
            persisted: high_water,
        };
        Ok(Self {
            key,
            held: Mutex::new(held),
        })
    }

    fn held(&self) -> std::sync::MutexGuard<'_, Held> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// The licence in force, valid or not.
    pub fn licence(&self) -> Option<Licence> {
        self.held().licence.clone()
    }

    /// The per-request check. Also advances the high-water mark.
    ///
    /// # Errors
    /// The high-water mark cannot be saved.
    pub fn is_valid(&self, db: &Database, now: OffsetDateTime) -> rusqlite::Result<bool> {
        let mut held = self.held();
        let rolled_back = held
            .high_water
            .is_some_and(|hw| now < hw - ROLLBACK_TOLERANCE);
        advance(&mut held, db, now)?;
        let Some(licence) = &held.licence else {
            return Ok(false);
        };
        Ok(!rolled_back && effective_now(&held, now) < licence.expires_at)
    }

    /// Verifies a new licence, saves it, then swaps it in. No restart needed.
    ///
    /// # Errors
    /// An invalid or already-expired licence, or a failed save; either way the current licence
    /// stays in force.
    ///
    /// # Panics
    /// Never: a licence of three strings always serialises.
    pub fn install(
        &self,
        db: &Database,
        doc: &LicenceDoc,
        now: OffsetDateTime,
    ) -> Result<Licence, InstallError> {
        let licence = verify(doc, self.key.as_ref())?;
        let mut held = self.held();
        if licence.expires_at <= effective_now(&held, now) {
            return Err(InvalidLicence("licence has already expired").into());
        }
        let high_water = effective_now(&held, now);
        let json = serde_json::to_string(&licence.doc).expect("a licence serialises");
        db.write(|tx| {
            tx.execute(
                "UPDATE license_state SET licence_json = ?, high_water = ? WHERE id = 1",
                params![json, isotime::to_utc_text(high_water)],
            )
        })?;
        held.licence = Some(licence.clone());
        held.high_water = Some(high_water);
        held.persisted = Some(high_water);
        Ok(licence)
    }
}

/// The later of the clock and the high-water mark: setting the clock back gains nothing.
fn effective_now(held: &Held, now: OffsetDateTime) -> OffsetDateTime {
    held.high_water.map_or(now, |hw| hw.max(now))
}

fn advance(held: &mut Held, db: &Database, now: OffsetDateTime) -> rusqlite::Result<()> {
    if held.high_water.is_none_or(|hw| now > hw) {
        held.high_water = Some(now);
    }
    let high_water = held.high_water.expect("just set");
    if held
        .persisted
        .is_none_or(|p| high_water - p >= PERSIST_EVERY)
    {
        db.write(|tx| {
            tx.execute(
                "UPDATE license_state SET high_water = ? WHERE id = 1",
                [isotime::to_utc_text(high_water)],
            )
        })?;
        held.persisted = Some(high_water);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license::verify::tests::{signed, signing_key};
    use time::macros::datetime;

    const NOW: OffsetDateTime = datetime!(2026-11-01 06:30 UTC);

    fn fresh() -> (Database, LicenceState) {
        let db = Database::in_memory().unwrap();
        let state = LicenceState::load(&db, Some(signing_key().verifying_key())).unwrap();
        (db, state)
    }

    #[test]
    fn no_licence_is_locked() {
        let (db, state) = fresh();
        assert!(!state.is_valid(&db, NOW).unwrap());
        assert_eq!(state.licence(), None);
    }

    #[test]
    fn valid_until_it_expires_then_locked_then_renewed() {
        let (db, state) = fresh();
        state
            .install(&db, &signed("Demo Bank", "2026-11-01T12:05:00+05:30"), NOW)
            .unwrap();
        assert!(state.is_valid(&db, NOW).unwrap());
        assert!(state.is_valid(&db, NOW + Duration::minutes(4)).unwrap());
        assert!(!state.is_valid(&db, NOW + Duration::minutes(5)).unwrap());
        state
            .install(
                &db,
                &signed("Demo Bank", "2027-11-01T12:00:00+05:30"),
                NOW + Duration::minutes(6),
            )
            .unwrap();
        assert!(state.is_valid(&db, NOW + Duration::minutes(6)).unwrap());
    }

    #[test]
    fn an_expired_or_bad_file_leaves_the_current_one() {
        let (db, state) = fresh();
        let good = signed("Demo Bank", "2027-01-01T00:00:00+05:30");
        state.install(&db, &good, NOW).unwrap();
        let old = signed("Old", "2026-01-01T00:00:00+05:30");
        assert!(matches!(
            state.install(&db, &old, NOW),
            Err(InstallError::Invalid(InvalidLicence(
                "licence has already expired"
            )))
        ));
        assert_eq!(state.licence().unwrap().doc, good);
    }

    #[test]
    fn setting_the_clock_back_does_not_help() {
        let (db, state) = fresh();
        state
            .install(&db, &signed("Demo Bank", "2026-11-01T12:05:00+05:30"), NOW)
            .unwrap();
        assert!(!state.is_valid(&db, NOW + Duration::minutes(10)).unwrap());
        // Back to a time before expiry: more than a minute behind the latest time seen.
        assert!(!state.is_valid(&db, NOW + Duration::minutes(2)).unwrap());
    }

    #[test]
    fn survives_a_restart() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("x.sqlite");
        let key = Some(signing_key().verifying_key());
        {
            let db = Database::open(&path).unwrap();
            let state = LicenceState::load(&db, key).unwrap();
            state
                .install(&db, &signed("Demo Bank", "2027-01-01T00:00:00+05:30"), NOW)
                .unwrap();
        }
        let db = Database::open(&path).unwrap();
        let state = LicenceState::load(&db, key).unwrap();
        assert_eq!(state.licence().unwrap().doc.licensee, "Demo Bank");
        assert!(state.is_valid(&db, NOW).unwrap());
        // Another server key does not accept the saved licence.
        let other = ed25519_dalek::SigningKey::from_bytes(&[9; 32]).verifying_key();
        assert_eq!(
            LicenceState::load(&db, Some(other)).unwrap().licence(),
            None
        );
    }
}
