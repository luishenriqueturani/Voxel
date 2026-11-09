//! Prompts recentes e favoritos.

use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use super::{Database, DbError};
use crate::output::{unix_now, utc_timestamp};

const RECENT_LIMIT: i64 = 30;

/// Prompt que a interface consegue recarregar, com ou sem estrela.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptEntry {
    pub prompt: String,
    pub negative_prompt: String,
    pub model_id: String,
    pub steps: i64,
    pub width: i64,
    pub height: i64,
    pub cfg: f64,
    pub seed: i64,
    pub pinned: bool,
    #[serde(default)]
    pub title: String,
}

/// Imagem de um prompt que ainda pode ser aberta.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptImage {
    pub path: String,
    pub seed: i64,
}

impl Database {
    /// Favoritos primeiro e, em seguida, os prompts mais novos que ainda não estão fixados.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn recent_prompts(&self) -> Result<Vec<PromptEntry>, DbError> {
        let mut listed = self.favorites()?;
        for entry in self.latest_prompts(RECENT_LIMIT)? {
            let already = listed
                .iter()
                .any(|item| item.pinned && same_prompt(item, &entry) && item.seed == entry.seed);
            if !already {
                listed.push(entry);
            }
        }
        Ok(listed)
    }

    /// Há prompts únicos além da primeira página.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn has_older_prompts(&self) -> Result<bool, DbError> {
        Ok(self.latest_prompts(RECENT_LIMIT + 1)?.len() > page_len())
    }

    /// Prompts que ficaram de fora da primeira página, do mais novo ao mais antigo.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn older_prompts(&self) -> Result<Vec<PromptEntry>, DbError> {
        let latest = self.latest_prompts(-1)?;
        Ok(latest.into_iter().skip(page_len()).collect())
    }

    /// Fixa ou solta o prompt. A estrela guarda a seed daquele momento.
    ///
    /// # Errors
    ///
    /// Falha se a escrita do banco falhar.
    pub fn set_favorite(&self, entry: &PromptEntry, pinned: bool) -> Result<(), DbError> {
        if pinned {
            self.lock()?.execute(
                "INSERT INTO favorites (
                    prompt, negative_prompt, model_id, steps, width, height, cfg, seed, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                ON CONFLICT (prompt, negative_prompt, model_id, steps, width, height, cfg)
                DO UPDATE SET seed = excluded.seed, created_at = excluded.created_at",
                (
                    &entry.prompt,
                    &entry.negative_prompt,
                    &entry.model_id,
                    entry.steps,
                    entry.width,
                    entry.height,
                    entry.cfg,
                    entry.seed,
                    utc_timestamp(unix_now()),
                ),
            )?;
            return Ok(());
        }
        self.lock()?.execute(
            "DELETE FROM favorites
             WHERE prompt = ?1 AND negative_prompt = ?2 AND model_id = ?3
               AND steps = ?4 AND width = ?5 AND height = ?6
               AND ABS(cfg - ?7) < 0.0001",
            (
                &entry.prompt,
                &entry.negative_prompt,
                &entry.model_id,
                entry.steps,
                entry.width,
                entry.height,
                entry.cfg,
            ),
        )?;
        Ok(())
    }

    /// Título curto já gerado para este prompt, se existir.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn prompt_title(&self, prompt: &str) -> Result<Option<String>, DbError> {
        let connection = self.lock()?;
        let title: Option<String> = connection
            .query_row(
                "SELECT title FROM prompt_titles WHERE prompt = ?1",
                [prompt],
                |row| row.get(0),
            )
            .optional()?;
        Ok(title.filter(|value| !value.is_empty()))
    }

    /// Grava o título curto. A imagem já pode ter saído.
    ///
    /// # Errors
    ///
    /// Falha se a escrita do banco falhar.
    pub fn set_prompt_title(&self, prompt: &str, title: &str) -> Result<(), DbError> {
        self.lock()?.execute(
            "INSERT INTO prompt_titles (prompt, title, created_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(prompt) DO UPDATE SET title = excluded.title, created_at = excluded.created_at",
            (prompt, title, utc_timestamp(unix_now())),
        )?;
        Ok(())
    }

    /// Imagens da geração mais recente desse prompt que ainda estão no disco.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn prompt_images(&self, entry: &PromptEntry) -> Result<Vec<PromptImage>, DbError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT path, seed FROM generations
             WHERE prompt = ?1 AND COALESCE(negative_prompt, '') = ?2 AND model_id = ?3
               AND steps = ?4 AND width = ?5 AND height = ?6
               AND ABS(cfg - ?7) < 0.0001
             ORDER BY id DESC",
        )?;
        let rows = statement
            .query_map(
                (
                    &entry.prompt,
                    &entry.negative_prompt,
                    &entry.model_id,
                    entry.steps,
                    entry.width,
                    entry.height,
                    entry.cfg,
                ),
                map_prompt_image,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        drop(connection);
        Ok(available_batch(&rows))
    }

    fn favorites(&self) -> Result<Vec<PromptEntry>, DbError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT f.prompt, f.negative_prompt, f.model_id, f.steps, f.width, f.height, f.cfg, f.seed,
                    COALESCE(t.title, '')
             FROM favorites f
             LEFT JOIN prompt_titles t ON t.prompt = f.prompt
             ORDER BY f.id DESC",
        )?;
        let rows = statement
            .query_map([], |row| map_prompt(row, true))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn latest_prompts(&self, limit: i64) -> Result<Vec<PromptEntry>, DbError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT g.prompt, COALESCE(g.negative_prompt, ''), g.model_id, g.steps, g.width, g.height, g.cfg, g.seed,
                    COALESCE(t.title, '')
             FROM generations g
             LEFT JOIN prompt_titles t ON t.prompt = g.prompt
             WHERE g.id IN (
                 SELECT MAX(id) FROM generations
                 GROUP BY prompt, COALESCE(negative_prompt, ''), model_id, steps, width, height, cfg
             )
             ORDER BY g.id DESC
             LIMIT ?1",
        )?;
        let rows = statement
            .query_map([limit], |row| map_prompt(row, false))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }
}

fn page_len() -> usize {
    usize::try_from(RECENT_LIMIT).unwrap_or(0)
}

fn map_prompt(row: &rusqlite::Row<'_>, pinned: bool) -> rusqlite::Result<PromptEntry> {
    Ok(PromptEntry {
        prompt: row.get(0)?,
        negative_prompt: row.get(1)?,
        model_id: row.get(2)?,
        steps: row.get(3)?,
        width: row.get(4)?,
        height: row.get(5)?,
        cfg: row.get(6)?,
        seed: row.get(7)?,
        pinned,
        title: row.get(8).unwrap_or_default(),
    })
}

fn map_prompt_image(row: &rusqlite::Row<'_>) -> rusqlite::Result<PromptImage> {
    Ok(PromptImage {
        path: row.get(0)?,
        seed: row.get(1)?,
    })
}

fn existing_folder(path: &str) -> Option<std::path::PathBuf> {
    let path = std::path::Path::new(path);
    path.is_file()
        .then(|| path.parent().map(std::path::Path::to_path_buf))
        .flatten()
}

fn available_batch(rows: &[PromptImage]) -> Vec<PromptImage> {
    let Some(folder) = rows.iter().find_map(|image| existing_folder(&image.path)) else {
        return Vec::new();
    };
    let mut images: Vec<PromptImage> = rows
        .iter()
        .filter(|image| existing_folder(&image.path).as_ref() == Some(&folder))
        .cloned()
        .collect();
    images.reverse();
    images
}

fn same_prompt(left: &PromptEntry, right: &PromptEntry) -> bool {
    left.prompt == right.prompt
        && left.negative_prompt == right.negative_prompt
        && left.model_id == right.model_id
        && left.steps == right.steps
        && left.width == right.width
        && left.height == right.height
        && (left.cfg - right.cfg).abs() < 0.000_1
}

#[cfg(test)]
mod tests {
    use super::Database;
    use crate::db::GenerationRecord;

    fn record(prompt: &str, seed: i64) -> GenerationRecord {
        GenerationRecord {
            path: format!("/tmp/{prompt}.png"),
            prompt: prompt.to_owned(),
            negative_prompt: None,
            model_id: "sd15".to_owned(),
            backend: "vulkan".to_owned(),
            seed,
            steps: 25,
            width: 512,
            height: 512,
            cfg: 7.0,
            elapsed_ms: 1,
            created_at: "2026-10-20T00:00:00Z".to_owned(),
        }
    }

    fn entry(prompt: &str, seed: i64) -> super::PromptEntry {
        super::PromptEntry {
            prompt: prompt.to_owned(),
            negative_prompt: String::new(),
            model_id: "sd15".to_owned(),
            steps: 25,
            width: 512,
            height: 512,
            cfg: 7.0,
            seed,
            pinned: false,
            title: String::new(),
        }
    }

    #[test]
    fn o_prompt_recente_fica_com_a_ultima_seed() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        database.record_generation(&record("gato", 1)).unwrap();
        database.record_generation(&record("gato", 2)).unwrap();
        database.record_generation(&record("casa", 3)).unwrap();

        let recent = database.recent_prompts().unwrap();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].prompt, "casa");
        assert_eq!(recent[1].prompt, "gato");
        assert_eq!(recent[1].seed, 2);
        database
            .set_prompt_title("gato", "felino no telhado")
            .unwrap();
        let titled = database.recent_prompts().unwrap();
        let gato = titled.iter().find(|item| item.prompt == "gato").unwrap();
        assert_eq!(gato.title, "felino no telhado");
    }

    #[test]
    fn o_favorito_permanece_depois_que_sai_da_janela_de_recentes() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        for index in 0..32 {
            database
                .record_generation(&record(&format!("p{index}"), index))
                .unwrap();
        }
        database.set_favorite(&entry("p0", 0), true).unwrap();

        let recent = database.recent_prompts().unwrap();
        assert!(recent.iter().any(|item| item.prompt == "p0" && item.pinned));
        assert!(!recent.iter().any(|item| item.prompt == "p1"));
        assert!(database.has_older_prompts().unwrap());
        let older = database.older_prompts().unwrap();
        assert!(older.iter().any(|item| item.prompt == "p1"));

        database.set_favorite(&entry("p0", 0), false).unwrap();
        let after = database.recent_prompts().unwrap();
        assert!(!after.iter().any(|item| item.prompt == "p0"));
    }

    #[test]
    fn o_clique_traz_as_imagens_da_geracao_que_ainda_existem() {
        let root = std::env::temp_dir().join(format!("voxel-hist-{}", std::process::id()));
        let old = root.join("antiga");
        let latest = root.join("nova");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&latest).unwrap();
        std::fs::write(old.join("imagem_01.png"), b"x").unwrap();
        std::fs::write(latest.join("imagem_01.png"), b"x").unwrap();
        std::fs::write(latest.join("imagem_02.png"), b"x").unwrap();

        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let mut kept = record("gato", 1);
        kept.path = old.join("imagem_01.png").display().to_string();
        database.record_generation(&kept).unwrap();
        for (name, seed) in [
            ("imagem_01.png", 4),
            ("imagem_02.png", 5),
            ("imagem_03.png", 6),
        ] {
            let mut image = record("gato", seed);
            image.path = latest.join(name).display().to_string();
            database.record_generation(&image).unwrap();
        }

        let images = database.prompt_images(&entry("gato", 6)).unwrap();
        assert_eq!(
            images.iter().map(|image| image.seed).collect::<Vec<_>>(),
            vec![4, 5]
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
