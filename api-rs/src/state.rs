//! What every handler shares: the database, the clock, the HMAC keyring and the licence.

use crate::clock::Clock;
use crate::core::identity::Keyring;
use crate::db::Database;
use crate::license::state::LicenceState;
use ed25519_dalek::VerifyingKey;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState(Arc<Inner>);

pub struct Inner {
    pub db: Database,
    pub clock: Arc<dyn Clock>,
    pub keys: Keyring,
    pub licence: LicenceState,
}

impl AppState {
    /// Loads the saved licence; `public_key` is the key licences must be signed with.
    ///
    /// # Errors
    /// The saved licence cannot be read.
    pub fn new(
        db: Database,
        clock: Arc<dyn Clock>,
        keys: Keyring,
        public_key: Option<VerifyingKey>,
    ) -> rusqlite::Result<Self> {
        let licence = LicenceState::load(&db, public_key)?;
        Ok(Self(Arc::new(Inner {
            db,
            clock,
            keys,
            licence,
        })))
    }
}

impl std::ops::Deref for AppState {
    type Target = Inner;

    fn deref(&self) -> &Inner {
        &self.0
    }
}
