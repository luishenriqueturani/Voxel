//! Histórico das imagens geradas. O arquivo em disco continua sendo a fonte da imagem.

use super::{Database, DbError};

/// Uma linha da tabela `generations`.
#[derive(Debug, Clone, PartialEq)]
pub struct GenerationRecord {
    pub path: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub model_id: String,
    pub backend: String,
    pub seed: i64,
    pub steps: i64,
    pub width: i64,
    pub height: i64,
    pub cfg: f64,
    pub elapsed_ms: i64,
    pub created_at: String,
}

impl Database {
    /// Grava a imagem que acabou de ser salva.
    ///
    /// # Errors
    ///
    /// Falha se o banco recusar a linha.
    pub fn record_generation(&self, record: &GenerationRecord) -> Result<(), DbError> {
        self.lock()?.execute(
            "INSERT INTO generations (
                path, prompt, negative_prompt, model_id, backend, seed, steps, width, height,
                cfg, elapsed_ms, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            (
                &record.path,
                &record.prompt,
                &record.negative_prompt,
                &record.model_id,
                &record.backend,
                record.seed,
                record.steps,
                record.width,
                record.height,
                record.cfg,
                record.elapsed_ms,
                &record.created_at,
            ),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Database, GenerationRecord};

    #[test]
    fn a_geracao_fica_no_banco() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        database
            .record_generation(&GenerationRecord {
                path: "/tmp/imagem_01.png".to_owned(),
                prompt: "um gato".to_owned(),
                negative_prompt: None,
                model_id: "sd15".to_owned(),
                backend: "vulkan".to_owned(),
                seed: 7,
                steps: 20,
                width: 512,
                height: 512,
                cfg: 7.0,
                elapsed_ms: 10,
                created_at: "2026-10-19T00:00:00Z".to_owned(),
            })
            .unwrap();
        let count: i64 = database
            .lock()
            .unwrap()
            .query_row("SELECT COUNT(*) FROM generations", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
