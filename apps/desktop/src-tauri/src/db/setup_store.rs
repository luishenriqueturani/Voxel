use rusqlite::OptionalExtension;

use super::{Database, DbError};

/// Uma combinação guardada porque quebrou nesta máquina.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompatibilityRow {
    pub model: String,
    pub backend: String,
    pub configuration: String,
    pub detail: String,
}

impl Database {
    /// Combinações que já quebraram nesta máquina.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn compatibility_failures(&self) -> Result<Vec<CompatibilityRow>, DbError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare("SELECT model, backend, configuration, detail FROM compatibility_failures")?;
        let rows = statement
            .query_map([], |row| {
                Ok(CompatibilityRow {
                    model: row.get(0)?,
                    backend: row.get(1)?,
                    configuration: row.get(2)?,
                    detail: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Guarda uma combinação que quebrou. Repetir a mesma chave atualiza o detalhe.
    ///
    /// # Errors
    ///
    /// Falha se a escrita falhar.
    pub fn record_failure(
        &self,
        model: &str,
        backend: &str,
        configuration: &str,
        detail: &str,
    ) -> Result<(), DbError> {
        let connection = self.lock()?;
        connection.execute(
            "INSERT INTO compatibility_failures (model, backend, configuration, detail)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(model, backend, configuration) DO UPDATE SET detail = excluded.detail",
            (model, backend, configuration, detail),
        )?;
        Ok(())
    }

    /// Lê um valor do setup. Ausente vira `None`.
    ///
    /// # Errors
    ///
    /// Falha se a leitura falhar.
    pub fn setup_flag(&self, key: &str) -> Result<Option<String>, DbError> {
        let connection = self.lock()?;
        Ok(connection
            .query_row(
                "SELECT value FROM setup_state WHERE key = ?1",
                [key],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Grava um valor do setup.
    ///
    /// # Errors
    ///
    /// Falha se a escrita falhar.
    pub fn set_setup_flag(&self, key: &str, value: &str) -> Result<(), DbError> {
        let connection = self.lock()?;
        connection.execute(
            "INSERT INTO setup_state (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            (key, value),
        )?;
        Ok(())
    }

    /// Apaga a memória de compatibilidade e o setup concluído.
    ///
    /// # Errors
    ///
    /// Falha se a escrita falhar. Nesse caso nada é apagado.
    pub fn reset_setup(&self) -> Result<(), DbError> {
        let connection = self.lock()?;
        let transaction = connection.unchecked_transaction()?;
        transaction.execute("DELETE FROM compatibility_failures", [])?;
        transaction.execute("DELETE FROM setup_state", [])?;
        transaction.commit()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::db::migrated;

    #[test]
    fn lembra_a_combinacao_que_quebrou_ate_refazer_o_setup() {
        let database = migrated();
        assert!(database.compatibility_failures().unwrap().is_empty());
        assert_eq!(database.setup_flag("completed").unwrap(), None);
        database
            .record_failure("sd15", "vulkan", "vae-f16", "SIGABRT")
            .unwrap();
        database
            .record_failure("sd15", "vulkan", "vae-f16", "de novo")
            .unwrap();
        database.set_setup_flag("completed", "1").unwrap();
        database.set_setup_flag("profile", "intermediario").unwrap();
        let failures = database.compatibility_failures().unwrap();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].detail, "de novo");
        assert_eq!(
            database.setup_flag("completed").unwrap().as_deref(),
            Some("1")
        );

        database.reset_setup().unwrap();
        assert!(database.compatibility_failures().unwrap().is_empty());
        assert_eq!(database.setup_flag("profile").unwrap(), None);
    }
}
