//! Biblioteca de templates de prompt: texto base, negativo e posição.

use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

use super::{Database, DbError};
use crate::output::{unix_now, utc_timestamp};

/// O texto base entra antes ou depois do prompt da geração.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PromptPlacement {
    Before,
    After,
}

impl PromptPlacement {
    fn parse(value: &str) -> Self {
        if value == "before" {
            Self::Before
        } else {
            Self::After
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::After => "after",
        }
    }
}

/// Template reutilizável na geração avulsa e nos fluxos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptTemplate {
    pub id: String,
    pub name: String,
    pub body: String,
    pub negative: String,
    pub placement: PromptPlacement,
}

const MARKER: &str = "{prompt}";

impl PromptTemplate {
    /// Junta o texto-base com o prompt do item, na posição gravada.
    #[must_use]
    pub fn compose(&self, item_prompt: &str, item_negative: &str) -> (String, String) {
        (
            compose_prompt(&self.body, item_prompt, self.placement),
            compose_negative(&self.negative, item_negative),
        )
    }
}

fn compose_prompt(body: &str, item_prompt: &str, placement: PromptPlacement) -> String {
    if body.contains(MARKER) {
        return body.replace(MARKER, item_prompt);
    }
    match placement {
        PromptPlacement::Before => join_parts(body, item_prompt),
        PromptPlacement::After => join_parts(item_prompt, body),
    }
}

fn compose_negative(template_negative: &str, item_negative: &str) -> String {
    join_parts(template_negative, item_negative)
}

fn join_parts(left: &str, right: &str) -> String {
    let first = left.trim();
    let second = right.trim();
    if first.is_empty() {
        return second.to_owned();
    }
    if second.is_empty() {
        return first.to_owned();
    }
    format!("{first}, {second}")
}

/// Campos de um template novo ou editado, sem identidade.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDraft {
    pub name: String,
    pub body: String,
    pub negative: String,
    pub placement: PromptPlacement,
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
            "SELECT id, name, body, negative, placement
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
            placement: draft.placement,
        };
        self.lock()?.execute(
            "INSERT INTO templates (id, name, body, negative, placement, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            (
                &template.id,
                &template.name,
                &template.body,
                &template.negative,
                template.placement.as_str(),
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
             SET name = ?2, body = ?3, negative = ?4, placement = ?5
             WHERE id = ?1",
            (
                id,
                &draft.name,
                &draft.body,
                &draft.negative,
                draft.placement.as_str(),
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
            placement: draft.placement,
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
            placement: existing.placement,
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

    /// Lê um template pela identidade.
    ///
    /// # Errors
    ///
    /// Falha se o template não existir.
    pub fn template(&self, id: &str) -> Result<PromptTemplate, DbError> {
        self.lock()?
            .query_row(
                "SELECT id, name, body, negative, placement FROM templates WHERE id = ?1",
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
    let placement: String = row.get(4)?;
    Ok(PromptTemplate {
        id: row.get(0)?,
        name: row.get(1)?,
        body: row.get(2)?,
        negative: row.get(3)?,
        placement: PromptPlacement::parse(&placement),
    })
}

fn checked(draft: &TemplateDraft) -> Result<TemplateDraft, DbError> {
    let name = draft.name.trim();
    if name.is_empty() {
        return Err(DbError::Invalid("nome vazio"));
    }
    Ok(TemplateDraft {
        name: name.to_owned(),
        body: draft.body.clone(),
        negative: draft.negative.clone(),
        placement: draft.placement,
    })
}

fn new_id() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    format!("{}-{sequence}", unix_now())
}

#[cfg(test)]
mod tests {
    use super::{Database, DbError, PromptPlacement, TemplateDraft};

    fn personagem() -> TemplateDraft {
        TemplateDraft {
            name: "Personagem para 3D".to_owned(),
            body: "corpo inteiro, T-pose".to_owned(),
            negative: "múltiplas pessoas".to_owned(),
            placement: PromptPlacement::After,
        }
    }

    #[test]
    fn cria_duplica_renomeia_e_exclui() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let created = database.create_template(&personagem()).unwrap();
        assert_eq!(created.body, "corpo inteiro, T-pose");
        assert_eq!(created.placement, PromptPlacement::After);

        let copy = database
            .duplicate_template(&created.id, "Personagem cópia")
            .unwrap();
        assert_ne!(copy.id, created.id);
        assert_eq!(copy.body, created.body);
        assert_eq!(copy.placement, PromptPlacement::After);

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
    fn a_edicao_troca_o_texto_e_a_posicao() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let created = database.create_template(&personagem()).unwrap();
        let mut draft = personagem();
        draft.placement = PromptPlacement::Before;
        draft.negative = "fundo complexo".to_owned();
        let updated = database.update_template(&created.id, &draft).unwrap();
        assert_eq!(updated.id, created.id);
        assert_eq!(updated.placement, PromptPlacement::Before);
        assert_eq!(updated.negative, "fundo complexo");
    }

    #[test]
    fn compoe_o_prompt_e_o_negativo() {
        let after = super::PromptTemplate {
            id: "a".to_owned(),
            name: "Personagem".to_owned(),
            body: "T-pose".to_owned(),
            negative: "fundo".to_owned(),
            placement: PromptPlacement::After,
        };
        assert_eq!(
            after.compose("ferreiro", "borrado"),
            ("ferreiro, T-pose".to_owned(), "fundo, borrado".to_owned())
        );
        let before = super::PromptTemplate {
            placement: PromptPlacement::Before,
            body: "foto de estúdio".to_owned(),
            ..after.clone()
        };
        assert_eq!(
            before.compose("ferreiro", "").0,
            "foto de estúdio, ferreiro"
        );
        let marked = super::PromptTemplate {
            body: "{prompt}, T-pose".to_owned(),
            ..after
        };
        assert_eq!(marked.compose("ferreiro", "").0, "ferreiro, T-pose");
    }
}
