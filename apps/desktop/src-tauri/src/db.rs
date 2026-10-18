use std::path::Path;
use std::sync::Mutex;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    sql: include_str!("migrations/001_settings.sql"),
}];

struct Migration {
    version: i64,
    sql: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "pt-BR")]
    Portuguese,
    #[serde(rename = "en")]
    English,
}

impl Language {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Portuguese => "pt-BR",
            Self::English => "en",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "pt-BR" => Some(Self::Portuguese),
            "en" => Some(Self::English),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "light" => Some(Self::Light),
            "dark" => Some(Self::Dark),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserSettings {
    pub language: Option<Language>,
    pub theme: Option<Theme>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SettingsUpdate {
    pub language: Option<Language>,
    pub theme: Option<Theme>,
}

#[derive(Debug)]
pub enum DbError {
    Sqlite(rusqlite::Error),
    Io(std::io::Error),
    LockPoisoned,
    Gap { expected: i64, found: i64 },
    NewerThanApp { version: i64 },
}

impl std::fmt::Display for DbError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlite(error) => write!(formatter, "sqlite: {error}"),
            Self::Io(error) => write!(formatter, "io: {error}"),
            Self::LockPoisoned => write!(formatter, "o banco está bloqueado"),
            Self::Gap { expected, found } => {
                write!(
                    formatter,
                    "migração {found} fora de ordem; esperada a {expected}"
                )
            }
            Self::NewerThanApp { version } => {
                write!(
                    formatter,
                    "o banco está na versão {version}, mais nova que este app"
                )
            }
        }
    }
}

impl std::error::Error for DbError {}

impl From<rusqlite::Error> for DbError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sqlite(error)
    }
}

impl From<std::io::Error> for DbError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

pub struct Database {
    connection: Mutex<Connection>,
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
    fn open_in_memory() -> Result<Self, DbError> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self, DbError> {
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    /// Aplica as migrações que este binário conhece e que ainda não rodaram.
    ///
    /// # Errors
    ///
    /// Falha se o banco estiver numa versão mais nova, se a sequência de versões tiver um buraco,
    /// ou se o `SQL` de uma migração for rejeitado. Nesse último caso a migração inteira é desfeita.
    pub fn migrate(&self) -> Result<(), DbError> {
        self.apply(MIGRATIONS)
    }

    fn apply(&self, migrations: &[Migration]) -> Result<(), DbError> {
        let mut current = self.current_version()?;
        let newest = migrations.last().map_or(0, |migration| migration.version);
        if current > newest {
            return Err(DbError::NewerThanApp { version: current });
        }
        for migration in migrations {
            if migration.version <= current {
                continue;
            }
            if migration.version != current + 1 {
                return Err(DbError::Gap {
                    expected: current + 1,
                    found: migration.version,
                });
            }
            self.apply_one(migration)?;
            current = migration.version;
        }
        Ok(())
    }

    fn apply_one(&self, migration: &Migration) -> Result<(), DbError> {
        let connection = self.lock()?;
        let transaction = connection.unchecked_transaction()?;
        transaction.execute_batch(migration.sql)?;
        transaction.execute(
            "INSERT INTO schema_migrations (version) VALUES (?1)",
            [migration.version],
        )?;
        transaction.commit()?;
        Ok(())
    }

    fn current_version(&self) -> Result<i64, DbError> {
        let connection = self.lock()?;
        let exists: i64 = connection.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'schema_migrations'",
            [],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Ok(0);
        }
        let mut statement =
            connection.prepare("SELECT version FROM schema_migrations ORDER BY version")?;
        let versions = statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<i64>, _>>()?;
        drop(statement);
        drop(connection);
        validate_versions(&versions)
    }

    /// Lê as configurações gravadas. Valor ausente ou desconhecido vira `None`.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn settings(&self) -> Result<UserSettings, DbError> {
        Ok(UserSettings {
            language: self.get("language")?.as_deref().and_then(Language::parse),
            theme: self.get("theme")?.as_deref().and_then(Theme::parse),
        })
    }

    /// Grava só os campos presentes e devolve o estado resultante.
    ///
    /// # Errors
    ///
    /// Falha se a escrita ou a leitura seguinte falhar.
    pub fn update_settings(&self, update: &SettingsUpdate) -> Result<UserSettings, DbError> {
        if let Some(language) = update.language {
            self.set("language", language.as_str())?;
        }
        if let Some(theme) = update.theme {
            self.set("theme", theme.as_str())?;
        }
        self.settings()
    }

    fn get(&self, key: &str) -> Result<Option<String>, DbError> {
        let connection = self.lock()?;
        Ok(connection
            .query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
                row.get(0)
            })
            .optional()?)
    }

    fn set(&self, key: &str, value: &str) -> Result<(), DbError> {
        let connection = self.lock()?;
        connection.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            (key, value),
        )?;
        Ok(())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, DbError> {
        self.connection.lock().map_err(|_| DbError::LockPoisoned)
    }
}

fn validate_versions(versions: &[i64]) -> Result<i64, DbError> {
    let mut ordered = versions.to_vec();
    ordered.sort_unstable();
    ordered.dedup();
    for (index, version) in ordered.iter().enumerate() {
        let expected = i64::try_from(index).unwrap_or(i64::MAX) + 1;
        if *version != expected {
            return Err(DbError::Gap {
                expected,
                found: *version,
            });
        }
    }
    Ok(ordered.last().copied().unwrap_or(0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrated() -> Database {
        let database = Database::open_in_memory().expect("banco em memória");
        database.migrate().expect("migração");
        database
    }

    #[test]
    fn migra_uma_vez_e_preserva_os_dados_na_segunda() {
        let database = migrated();
        database
            .update_settings(&SettingsUpdate {
                language: Some(Language::English),
                theme: None,
            })
            .unwrap();

        database.migrate().unwrap();

        assert_eq!(
            database.settings().unwrap(),
            UserSettings {
                language: Some(Language::English),
                theme: None
            }
        );
    }

    #[test]
    fn configuracoes_novas_sao_ausentes() {
        let settings = migrated().settings().unwrap();
        assert_eq!(
            settings,
            UserSettings {
                language: None,
                theme: None
            }
        );
    }

    #[test]
    fn grava_e_rele_idioma_e_tema() {
        let database = migrated();
        let saved = database
            .update_settings(&SettingsUpdate {
                language: Some(Language::Portuguese),
                theme: Some(Theme::Dark),
            })
            .unwrap();

        assert_eq!(saved.language, Some(Language::Portuguese));
        assert_eq!(saved.theme, Some(Theme::Dark));
        assert_eq!(database.settings().unwrap(), saved);
    }

    #[test]
    fn atualizacao_parcial_nao_apaga_o_outro_campo() {
        let database = migrated();
        database
            .update_settings(&SettingsUpdate {
                language: Some(Language::English),
                theme: None,
            })
            .unwrap();
        database
            .update_settings(&SettingsUpdate {
                language: None,
                theme: Some(Theme::Light),
            })
            .unwrap();

        assert_eq!(
            database.settings().unwrap(),
            UserSettings {
                language: Some(Language::English),
                theme: Some(Theme::Light)
            }
        );
    }

    #[test]
    fn valor_desconhecido_e_ignorado_na_leitura() {
        let database = migrated();
        database.set("language", "klingon").unwrap();
        database.set("theme", "sepia").unwrap();

        assert_eq!(
            database.settings().unwrap(),
            UserSettings {
                language: None,
                theme: None
            }
        );
    }

    #[test]
    fn arquivo_reabre_com_os_mesmos_dados() {
        let directory = std::env::temp_dir().join(format!("voxel-db-{}", std::process::id()));
        let path = directory.join("voxel.db");
        let _ = std::fs::remove_dir_all(&directory);

        let database = Database::open(&path).unwrap();
        database.migrate().unwrap();
        database
            .update_settings(&SettingsUpdate {
                language: None,
                theme: Some(Theme::Dark),
            })
            .unwrap();
        drop(database);

        let reopened = Database::open(&path).unwrap();
        reopened.migrate().unwrap();
        assert_eq!(reopened.settings().unwrap().theme, Some(Theme::Dark));
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn migracao_com_sql_invalido_nao_avanca_a_versao() {
        let database = Database::open_in_memory().unwrap();
        database
            .apply(&[Migration {
                version: 1,
                sql: include_str!("migrations/001_settings.sql"),
            }])
            .unwrap();

        let broken = [
            Migration {
                version: 1,
                sql: "",
            },
            Migration {
                version: 2,
                sql: "ISTO NAO E SQL;",
            },
        ];
        assert!(database.apply(&broken).is_err());
        assert_eq!(database.current_version().unwrap(), 1);
        assert!(database.settings().is_ok());
    }

    #[test]
    fn recusa_banco_mais_novo_que_o_app() {
        let database = migrated();
        database
            .apply_one(&Migration {
                version: 2,
                sql: "SELECT 1;",
            })
            .unwrap();

        assert!(matches!(
            database.migrate(),
            Err(DbError::NewerThanApp { version: 2 })
        ));
    }

    #[test]
    fn recusa_salto_de_versao() {
        let database = Database::open_in_memory().unwrap();
        let error = database
            .apply(&[Migration {
                version: 2,
                sql: "SELECT 1;",
            }])
            .unwrap_err();

        assert!(matches!(
            error,
            DbError::Gap {
                expected: 1,
                found: 2
            }
        ));
    }

    #[test]
    fn parse_de_idioma_e_tema() {
        assert_eq!(Language::parse("pt-BR"), Some(Language::Portuguese));
        assert_eq!(Language::parse("en"), Some(Language::English));
        assert_eq!(Language::parse("es"), None);
        assert_eq!(Language::Portuguese.as_str(), "pt-BR");
        assert_eq!(Theme::parse("dark"), Some(Theme::Dark));
        assert_eq!(Theme::parse("azul"), None);
        assert_eq!(Theme::Light.as_str(), "light");
    }

    #[test]
    fn json_parcial_nao_exige_os_dois_campos() {
        let update: SettingsUpdate = serde_json::from_str(r#"{"theme":"light"}"#).unwrap();
        assert_eq!(update.language, None);
        assert_eq!(update.theme, Some(Theme::Light));
    }

    #[test]
    fn versoes_gravadas_fora_de_ordem_sao_recusadas() {
        let database = migrated();
        database
            .apply_one(&Migration {
                version: 3,
                sql: "SELECT 1;",
            })
            .unwrap();

        assert!(matches!(
            database.current_version(),
            Err(DbError::Gap {
                expected: 2,
                found: 3
            })
        ));
    }

    #[test]
    fn lista_vazia_de_migracoes_nao_altera_o_banco() {
        let database = Database::open_in_memory().unwrap();
        database.apply(&[]).unwrap();
        assert_eq!(database.current_version().unwrap(), 0);
    }

    #[test]
    fn mensagens_de_erro() {
        assert!(DbError::LockPoisoned.to_string().contains("bloqueado"));
        assert!(
            DbError::Gap {
                expected: 1,
                found: 3
            }
            .to_string()
            .contains('3')
        );
        assert!(
            DbError::NewerThanApp { version: 9 }
                .to_string()
                .contains('9')
        );

        let directory = std::env::temp_dir().join(format!("voxel-db-io-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        let blocker = directory.join("arquivo");
        std::fs::write(&blocker, "x").unwrap();
        let io_error = Database::open(&blocker.join("voxel.db")).unwrap_err();
        assert!(io_error.to_string().contains("io"));

        let database = Database::open_in_memory().unwrap();
        let sqlite_error = database
            .apply(&[Migration {
                version: 1,
                sql: "ISTO NAO E SQL;",
            }])
            .unwrap_err();
        assert!(sqlite_error.to_string().contains("sqlite"));
        let _ = std::fs::remove_dir_all(directory);
    }
}
