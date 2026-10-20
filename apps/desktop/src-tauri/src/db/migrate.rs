use super::{Database, DbError};

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        sql: include_str!("../migrations/001_settings.sql"),
    },
    Migration {
        version: 2,
        sql: include_str!("../migrations/002_setup.sql"),
    },
    Migration {
        version: 3,
        sql: include_str!("../migrations/003_generations.sql"),
    },
    Migration {
        version: 4,
        sql: include_str!("../migrations/004_favorites.sql"),
    },
];

struct Migration {
    version: i64,
    sql: &'static str,
}

impl Database {
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
    use super::{Database, DbError, Migration};
    use crate::db::migrated;

    #[test]
    fn migracao_com_sql_invalido_nao_avanca_a_versao() {
        let database = Database::open_in_memory().unwrap();
        database
            .apply(&[Migration {
                version: 1,
                sql: include_str!("../migrations/001_settings.sql"),
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
                version: 5,
                sql: "SELECT 1;",
            })
            .unwrap();

        assert!(matches!(
            database.migrate(),
            Err(DbError::NewerThanApp { version: 5 })
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
    fn versoes_gravadas_fora_de_ordem_sao_recusadas() {
        let database = migrated();
        database
            .apply_one(&Migration {
                version: 6,
                sql: "SELECT 1;",
            })
            .unwrap();

        assert!(matches!(
            database.current_version(),
            Err(DbError::Gap {
                expected: 5,
                found: 6
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
