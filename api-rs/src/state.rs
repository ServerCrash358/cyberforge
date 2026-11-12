//! What every handler shares: the database, the clock, the HMAC keyring, the licence and the
//! settings that only matter on a public server.

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
    pub options: Options,
}

/// Settings for a server on the public internet. The defaults are the PRD's behaviour.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// When set, `POST /admin/reset` needs this value in the `x-admin-token` header. The PRD wants
    /// reset open for the judges' tool; on a public server anyone could otherwise erase it.
    pub admin_token: Option<String>,
    /// Behind a reverse proxy (Caddy), the client's address is the last `X-Forwarded-For` entry,
    /// the one the proxy added; the connection itself always comes from the proxy.
    pub trust_proxy: bool,
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
        Self::with_options(db, clock, keys, public_key, Options::default())
    }

    /// Like [`AppState::new`], with the public-server settings.
    ///
    /// # Errors
    /// The saved licence cannot be read.
    pub fn with_options(
        db: Database,
        clock: Arc<dyn Clock>,
        keys: Keyring,
        public_key: Option<VerifyingKey>,
        options: Options,
    ) -> rusqlite::Result<Self> {
        let licence = LicenceState::load(&db, public_key)?;
        Ok(Self(Arc::new(Inner {
            db,
            clock,
            keys,
            licence,
            options,
        })))
    }
}

impl std::ops::Deref for AppState {
    type Target = Inner;

    fn deref(&self) -> &Inner {
        &self.0
    }
}
