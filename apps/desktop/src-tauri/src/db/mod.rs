mod error;
mod generations;
mod migrate;
mod settings;
mod setup_store;

pub use error::DbError;
pub use generations::GenerationRecord;
pub use settings::{SettingsUpdate, UserSettings};

use std::path::Path;
use std::sync::{Arc, Mutex};

use rusqlite::Connection;

#[derive(Clone)]
pub struct Database {
    connection: Arc<Mutex<Connection>>,
}

impl std::fmt::Debug for Database {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Database")
    }
}

impl Database {
    /// Abre (e cria) o banco no caminho informado.
    ///
    /// # Errors
    ///
    /// Falha se o diretório não puder ser criado ou se o `SQLite` recusar o arquivo.
    pub fn open(path: &Path) -> Result<Self, DbError> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        Self::from_connection(Connection::open(path)?)
    }

    /// Abre um banco só na memória, para testes.
    ///
    /// # Errors
    ///
    /// Falha se o `SQLite` não conseguir alocar o banco.
    #[cfg(test)]
    pub(crate) fn open_in_memory() -> Result<Self, DbError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self, DbError> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        Ok(Self {
            connection: Arc::new(Mutex::new(connection)),
        })
    }

    pub(crate) fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, DbError> {
        self.connection.lock().map_err(|_| DbError::LockPoisoned)
    }
}

#[cfg(test)]
fn migrated() -> Database {
    let database = Database::open_in_memory().expect("banco em memória");
    database.migrate().expect("migração");
    database
}
