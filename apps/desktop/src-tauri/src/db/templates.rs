//! Biblioteca de templates de prompt. O marcador `{prompt}` recebe o texto do item.

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use super::{Database, DbError};
use crate::output::{unix_now, utc_timestamp};

/// Template reutilizável por qualquer fluxo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub body: String,
    pub negative: String,
    #[serde(rename = "model2d")]
    pub model_2d: String,
    #[serde(rename = "model3d")]
    pub model_3d: String,
    pub resolution: i64,
    pub aspect: String,
    pub steps: i64,
    pub cfg: f64,
    pub seed: String,
    pub count: i64,
    pub postprocess: bool,
}

/// Campos de um template novo ou editado, sem identidade.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDraft {
    pub name: String,
    pub body: String,
    pub negative: String,
    #[serde(rename = "model2d")]
    pub model_2d: String,
    #[serde(rename = "model3d")]
    pub model_3d: String,
    pub resolution: i64,
    pub aspect: String,
    pub steps: i64,
    pub cfg: f64,
    pub seed: String,
    pub count: i64,
    pub postprocess: bool,
}

impl Database {
    /// Templates em ordem de nome.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn list_templates(&self) -> Result<Vec<PromptTemplate>, DbError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT id, name, body, negative, model_2d, model_3d, resolution, aspect, steps, cfg,
                    seed, count, postprocess
             FROM templates
             ORDER BY name COLLATE NOCASE, id",
        )?;
        let rows = statement.query_map([], read_template)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
    }

    /// Grava um template novo.
    ///
    /// # Errors
    ///
    /// Falha se os campos forem inválidos ou se o banco recusar a linha.
    pub fn create_template(&self, draft: &TemplateDraft) -> Result<PromptTemplate, DbError> {
        let draft = checked(draft)?;
        let template = PromptTemplate {
            id: new_id(),
            name: draft.name,
            body: draft.body,
            negative: draft.negative,
            model_2d: draft.model_2d,
            model_3d: draft.model_3d,
            resolution: draft.resolution,
            aspect: draft.aspect,
            steps: draft.steps,
            cfg: draft.cfg,
            seed: draft.seed,
            count: draft.count,
            postprocess: draft.postprocess,
        };
        self.lock()?.execute(
            "INSERT INTO templates (
                id, name, body, negative, model_2d, model_3d, resolution, aspect, steps, cfg,
                seed, count, postprocess, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            (
                &template.id,
                &template.name,
                &template.body,
                &template.negative,
                &template.model_2d,
                &template.model_3d,
                template.resolution,
                &template.aspect,
                template.steps,
                template.cfg,
                &template.seed,
                template.count,
                i64::from(template.postprocess),
                utc_timestamp(unix_now()),
            ),
        )?;
        Ok(template)
    }

    /// Substitui os campos de um template que já existe.
    ///
    /// # Errors
    ///
    /// Falha se o template não existir, se os campos forem inválidos ou se o banco recusar a escrita.
    pub fn update_template(
        &self,
        id: &str,
        draft: &TemplateDraft,
    ) -> Result<PromptTemplate, DbError> {
        let draft = checked(draft)?;
        let changed = self.lock()?.execute(
            "UPDATE templates
             SET name = ?2, body = ?3, negative = ?4, model_2d = ?5, model_3d = ?6,
                 resolution = ?7, aspect = ?8, steps = ?9, cfg = ?10, seed = ?11, count = ?12,
                 postprocess = ?13
             WHERE id = ?1",
            (
                id,
                &draft.name,
                &draft.body,
                &draft.negative,
                &draft.model_2d,
                &draft.model_3d,
                draft.resolution,
                &draft.aspect,
                draft.steps,
                draft.cfg,
                &draft.seed,
                draft.count,
                i64::from(draft.postprocess),
            ),
        )?;
        if changed == 0 {
            return Err(DbError::Missing);
        }
        Ok(PromptTemplate {
            id: id.to_owned(),
            name: draft.name,
            body: draft.body,
            negative: draft.negative,
            model_2d: draft.model_2d,
            model_3d: draft.model_3d,
            resolution: draft.resolution,
            aspect: draft.aspect,
            steps: draft.steps,
            cfg: draft.cfg,
            seed: draft.seed,
            count: draft.count,
            postprocess: draft.postprocess,
        })
    }

    /// Copia um template para um nome novo.
    ///
    /// # Errors
    ///
    /// Falha se a origem não existir ou se o nome novo for inválido.
    pub fn duplicate_template(&self, id: &str, name: &str) -> Result<PromptTemplate, DbError> {
        let existing = self.template(id)?;
        self.create_template(&TemplateDraft {
            name: name.to_owned(),
            body: existing.body,
            negative: existing.negative,
            model_2d: existing.model_2d,
            model_3d: existing.model_3d,
            resolution: existing.resolution,
            aspect: existing.aspect,
            steps: existing.steps,
            cfg: existing.cfg,
            seed: existing.seed,
            count: existing.count,
            postprocess: existing.postprocess,
        })
    }

    /// Troca só o nome.
    ///
    /// # Errors
    ///
    /// Falha se o template não existir ou se o nome vier vazio.
    pub fn rename_template(&self, id: &str, name: &str) -> Result<PromptTemplate, DbError> {
        let mut template = self.template(id)?;
        let name = name.trim();
        if name.is_empty() {
            return Err(DbError::Invalid("nome vazio"));
        }
        let changed = self
            .lock()?
            .execute("UPDATE templates SET name = ?2 WHERE id = ?1", (id, name))?;
        if changed == 0 {
            return Err(DbError::Missing);
        }
        name.clone_into(&mut template.name);
        Ok(template)
    }

    /// Apaga um template.
    ///
    /// # Errors
    ///
    /// Falha se o template não existir ou se o banco recusar a exclusão.
    pub fn delete_template(&self, id: &str) -> Result<(), DbError> {
        let changed = self
            .lock()?
            .execute("DELETE FROM templates WHERE id = ?1", [id])?;
        if changed == 0 {
            return Err(DbError::Missing);
        }
        Ok(())
    }

    fn template(&self, id: &str) -> Result<PromptTemplate, DbError> {
        self.lock()?
            .query_row(
                "SELECT id, name, body, negative, model_2d, model_3d, resolution, aspect, steps,
                        cfg, seed, count, postprocess
                 FROM templates
                 WHERE id = ?1",
                [id],
                read_template,
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => DbError::Missing,
                other => DbError::from(other),
            })
    }
}

fn read_template(row: &rusqlite::Row<'_>) -> rusqlite::Result<PromptTemplate> {
    let postprocess: i64 = row.get(12)?;
    Ok(PromptTemplate {
        id: row.get(0)?,
        name: row.get(1)?,
        body: row.get(2)?,
        negative: row.get(3)?,
        model_2d: row.get(4)?,
        model_3d: row.get(5)?,
        resolution: row.get(6)?,
        aspect: row.get(7)?,
        steps: row.get(8)?,
        cfg: row.get(9)?,
        seed: row.get(10)?,
        count: row.get(11)?,
        postprocess: postprocess != 0,
    })
}

fn checked(draft: &TemplateDraft) -> Result<TemplateDraft, DbError> {
    let name = draft.name.trim();
    if name.is_empty() {
        return Err(DbError::Invalid("nome vazio"));
    }
    if draft.model_2d.trim().is_empty() || draft.model_3d.trim().is_empty() {
        return Err(DbError::Invalid("modelo vazio"));
    }
    if !matches!(draft.resolution, 512 | 768 | 1024) {
        return Err(DbError::Invalid("resolução inválida"));
    }
    if !known_aspect(&draft.aspect) {
        return Err(DbError::Invalid("proporção inválida"));
    }
    if !matches!(draft.steps, 1..=500) {
        return Err(DbError::Invalid("passos inválidos"));
    }
    if !draft.cfg.is_finite() || !(0.0..=30.0).contains(&draft.cfg) {
        return Err(DbError::Invalid("cfg inválido"));
    }
    if !draft.seed.is_empty() && draft.seed.parse::<i64>().is_err() {
        return Err(DbError::Invalid("seed inválida"));
    }
    if draft.count < 1 {
        return Err(DbError::Invalid("quantidade inválida"));
    }
    Ok(TemplateDraft {
        name: name.to_owned(),
        body: draft.body.clone(),
        negative: draft.negative.clone(),
        model_2d: draft.model_2d.trim().to_owned(),
        model_3d: draft.model_3d.trim().to_owned(),
        resolution: draft.resolution,
        aspect: draft.aspect.clone(),
        steps: draft.steps,
        cfg: draft.cfg,
        seed: draft.seed.clone(),
        count: draft.count,
        postprocess: draft.postprocess,
    })
}

fn known_aspect(value: &str) -> bool {
    matches!(
        value,
        "1:1" | "4:3" | "3:2" | "16:9" | "21:9" | "3:4" | "2:3" | "9:16" | "9:21"
    )
}

fn new_id() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    format!("{}-{sequence}", unix_now())
}

#[cfg(test)]
mod tests {
    use super::{Database, DbError, TemplateDraft};

    fn personagem() -> TemplateDraft {
        TemplateDraft {
            name: "Personagem para 3D".to_owned(),
            body: "{prompt}, corpo inteiro, T-pose".to_owned(),
            negative: "múltiplas pessoas".to_owned(),
            model_2d: "sd15".to_owned(),
            model_3d: "triposr".to_owned(),
            resolution: 512,
            aspect: "1:1".to_owned(),
            steps: 20,
            cfg: 7.0,
            seed: String::new(),
            count: 1,
            postprocess: true,
        }
    }

    #[test]
    fn cria_duplica_renomeia_e_exclui() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let created = database.create_template(&personagem()).unwrap();
        assert!(created.body.contains("{prompt}"));
        assert!(created.postprocess);

        let copy = database
            .duplicate_template(&created.id, "Personagem cópia")
            .unwrap();
        assert_ne!(copy.id, created.id);
        assert_eq!(copy.body, created.body);
        assert_eq!(copy.model_2d, "sd15");
        assert_eq!(copy.model_3d, "triposr");
        assert!(copy.postprocess);

        let renamed = database.rename_template(&created.id, "  Herói  ").unwrap();
        assert_eq!(renamed.name, "Herói");
        let listed = database.list_templates().unwrap();
        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].name, "Herói");

        database.delete_template(&copy.id).unwrap();
        assert_eq!(database.list_templates().unwrap().len(), 1);
        assert!(matches!(
            database.delete_template(&copy.id),
            Err(DbError::Missing)
        ));
    }

    #[test]
    fn recusa_nome_vazio_e_origem_ausente() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let mut draft = personagem();
        draft.name = "   ".to_owned();
        assert!(matches!(
            database.create_template(&draft),
            Err(DbError::Invalid("nome vazio"))
        ));
        assert!(matches!(
            database.duplicate_template("ausente", "Cópia"),
            Err(DbError::Missing)
        ));
    }

    #[test]
    fn a_edicao_troca_os_parametros_e_o_pos_processamento() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let created = database.create_template(&personagem()).unwrap();
        let mut draft = personagem();
        draft.steps = 28;
        draft.postprocess = false;
        draft.negative = "fundo complexo".to_owned();
        let updated = database.update_template(&created.id, &draft).unwrap();
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.steps, 28);
        assert!(!updated.postprocess);
        assert_eq!(updated.negative, "fundo complexo");
    }
}
