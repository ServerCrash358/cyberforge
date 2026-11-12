//! The SQLite database in WAL mode and its migrations; the one place that opens it.
//!
//! One shared connection behind a mutex, like the Python API: each request holds it for a few
//! milliseconds, and SQLite allows one writer at a time anyway.

use rusqlite::{Connection, Transaction, TransactionBehavior};
use std::path::Path;
use std::sync::{Mutex, PoisonError};

/// `migrations/*.sql`, built into the binary, applied in this order and recorded by name.
const MIGRATIONS: [(&str, &str); 7] = [
    ("0001_init", include_str!("../migrations/0001_init.sql")),
    (
        "0002_reports_cases",
        include_str!("../migrations/0002_reports_cases.sql"),
    ),
    (
        "0003_license_state",
        include_str!("../migrations/0003_license_state.sql"),
    ),
    ("0004_trust", include_str!("../migrations/0004_trust.sql")),
    (
        "0005_challenges",
        include_str!("../migrations/0005_challenges.sql"),
    ),
    ("0006_events", include_str!("../migrations/0006_events.sql")),
    (
        "0007_event_age",
        include_str!("../migrations/0007_event_age.sql"),
    ),
];

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Opens (creating if needed) the database and applies any new migrations.
    ///
    /// # Errors
    /// The file cannot be opened, or a migration fails (it is then rolled back, not recorded).
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)
                .map_err(|_| rusqlite::Error::InvalidPath(parent.to_path_buf()))?;
        }
        Self::setup(Connection::open(path)?)
    }

    /// An in-memory database for tests.
    ///
    /// # Errors
    /// A migration fails.
    pub fn in_memory() -> rusqlite::Result<Self> {
        Self::setup(Connection::open_in_memory()?)
    }

    fn setup(mut conn: Connection) -> rusqlite::Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        conn.pragma_update(None, "busy_timeout", 5000)?;
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        apply_migrations(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Copies the whole database to `target` while the server keeps running (SQLite's online
    /// backup), for the nightly backup on the server.
    ///
    /// # Errors
    /// The target cannot be written.
    pub fn backup_to(&self, target: &Path) -> rusqlite::Result<()> {
        let conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        conn.backup(
            rusqlite::MAIN_DB,
            target,
            None::<fn(rusqlite::backup::Progress)>,
        )
    }

    /// Runs `f` with the connection, for reads.
    pub fn read<T>(&self, f: impl FnOnce(&Connection) -> T) -> T {
        f(&self.conn.lock().unwrap_or_else(PoisonError::into_inner))
    }

    /// Runs `f` in a transaction: committed when it returns `Ok`, rolled back otherwise.
    ///
    /// # Errors
    /// The transaction cannot start or commit, or `f` fails.
    pub fn write<T, E: From<rusqlite::Error>>(
        &self,
        f: impl FnOnce(&Transaction<'_>) -> Result<T, E>,
    ) -> Result<T, E> {
        let mut conn = self.conn.lock().unwrap_or_else(PoisonError::into_inner);
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let value = f(&tx)?;
        tx.commit()?;
        Ok(value)
    }
}

/// Every migration not yet recorded, each in its own transaction. Returns the names applied.
fn apply_migrations(conn: &mut Connection) -> rusqlite::Result<Vec<&'static str>> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations ( version TEXT PRIMARY KEY, \
         applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')) )",
        [],
    )?;
    let mut applied = Vec::new();
    for (version, sql) in MIGRATIONS {
        let done: bool = conn.query_row(
            "SELECT EXISTS (SELECT 1 FROM schema_migrations WHERE version = ?)",
            [version],
            |r| r.get(0),
        )?;
        if done {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version) VALUES (?)",
            [version],
        )?;
        tx.commit()?;
        applied.push(version);
    }
    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_run_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("x.sqlite");
        let db = Database::open(&path).unwrap();
        let count: i64 = db
            .read(|c| c.query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(count, 7);
        drop(db);
        let mut conn = Connection::open(&path).unwrap();
        assert_eq!(apply_migrations(&mut conn).unwrap(), Vec::<&str>::new());
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
    }

    #[test]
    fn a_backup_is_a_complete_copy() {
        let dir = tempfile::tempdir().unwrap();
        let db = Database::open(&dir.path().join("live.sqlite")).unwrap();
        let copy = dir.path().join("copy.sqlite");
        db.backup_to(&copy).unwrap();
        let restored = Database::open(&copy).unwrap();
        let count: i64 = restored
            .read(|c| c.query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(count, 7);
    }

    #[test]
    fn a_failed_write_rolls_back() {
        let db = Database::in_memory().unwrap();
        let result: rusqlite::Result<()> = db.write(|tx| {
            tx.execute("DELETE FROM license_state", [])?;
            tx.execute("SELECT nonsense FROM nowhere", [])?;
            Ok(())
        });
        assert!(result.is_err());
        let rows: i64 = db
            .read(|c| c.query_row("SELECT COUNT(*) FROM license_state", [], |r| r.get(0)))
            .unwrap();
        assert_eq!(rows, 1);
    }
}
