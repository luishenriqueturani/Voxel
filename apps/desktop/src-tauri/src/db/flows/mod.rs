//! Biblioteca de fluxos e arquivo JSON do menu Arquivo.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

use super::{Database, DbError};
use crate::output::{unix_now, utc_timestamp};

const FLOW_FILE_VERSION: i64 = 1;

/// Modo 3D de um item: não gera, gera todas as imagens, ou escolhe depois.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MeshMode {
    None,
    All,
    Later,
}

/// Parâmetros do template que um item pode trocar.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowOverrides {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub negative: Option<String>,
    #[serde(rename = "model2d", default, skip_serializing_if = "Option::is_none")]
    pub model_2d: Option<String>,
    #[serde(rename = "model3d", default, skip_serializing_if = "Option::is_none")]
    pub model_3d: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aspect: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub steps: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cfg: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seed: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub postprocess: Option<bool>,
}

/// Um item da lista do fluxo.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowItem {
    pub id: String,
    pub prompt: String,
    pub count: i64,
    pub mesh: MeshMode,
    #[serde(rename = "templateId", default)]
    pub template_id: Option<String>,
    pub overrides: FlowOverrides,
}

/// Campos gravados, sem a identidade do banco.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowDraft {
    pub name: String,
    #[serde(rename = "templateId")]
    pub template_id: Option<String>,
    pub path: Option<String>,
    pub items: Vec<FlowItem>,
}

/// Fluxo completo, com os itens na ordem.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlowDocument {
    pub id: String,
    pub name: String,
    #[serde(rename = "templateId")]
    pub template_id: Option<String>,
    pub path: Option<String>,
    pub items: Vec<FlowItem>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct FlowFile {
    version: i64,
    name: String,
    template_id: Option<String>,
    items: Vec<FlowFileItem>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct FlowFileItem {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    prompt: String,
    #[serde(default = "one")]
    count: i64,
    mesh: MeshMode,
    #[serde(default)]
    template_id: Option<String>,
    #[serde(default)]
    overrides: FlowOverrides,
}

fn one() -> i64 {
    1
}

impl Database {
    /// Fluxos em ordem de nome, com os itens na posição gravada.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn list_flows(&self) -> Result<Vec<FlowDocument>, DbError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare(
            "SELECT id, name, template_id, path FROM flows ORDER BY name COLLATE NOCASE, id",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?;
        let headers = rows.collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        let mut grouped = load_items(&connection)?;
        Ok(headers
            .into_iter()
            .map(|(id, name, template_id, path)| FlowDocument {
                items: grouped.remove(&id).unwrap_or_default(),
                id,
                name,
                template_id,
                path,
            })
            .collect())
    }

    /// Grava um fluxo novo.
    ///
    /// # Errors
    ///
    /// Falha se os campos forem inválidos ou se o banco recusar a linha.
    pub fn create_flow(&self, draft: &FlowDraft) -> Result<FlowDocument, DbError> {
        let draft = checked(draft)?;
        let document = FlowDocument {
            id: new_id(),
            name: draft.name,
            template_id: draft.template_id,
            path: draft.path,
            items: draft.items,
        };
        self.write_flow(&document, true)?;
        Ok(document)
    }

    /// Substitui nome, template, caminho e itens.
    ///
    /// # Errors
    ///
    /// Falha se o fluxo não existir ou se os campos forem inválidos.
    pub fn update_flow(&self, id: &str, draft: &FlowDraft) -> Result<FlowDocument, DbError> {
        let draft = checked(draft)?;
        let document = FlowDocument {
            id: id.to_owned(),
            name: draft.name,
            template_id: draft.template_id,
            path: draft.path,
            items: draft.items,
        };
        self.write_flow(&document, false)?;
        Ok(document)
    }

    /// Apaga um fluxo e os itens.
    ///
    /// # Errors
    ///
    /// Falha se o fluxo não existir.
    pub fn delete_flow(&self, id: &str) -> Result<(), DbError> {
        let changed = self
            .lock()?
            .execute("DELETE FROM flows WHERE id = ?1", [id])?;
        if changed == 0 {
            return Err(DbError::Missing);
        }
        Ok(())
    }

    /// Lê um JSON de fluxo e grava no banco, reusando a linha do mesmo caminho.
    ///
    /// # Errors
    ///
    /// Falha se o arquivo for inválido ou se o banco recusar a escrita.
    pub fn import_flow_file(&self, path: &Path) -> Result<FlowDocument, DbError> {
        let text = std::fs::read_to_string(path)?;
        let mut draft = parse_flow_file(&text)?;
        let stored = path
            .to_str()
            .ok_or(DbError::Invalid("caminho inválido"))?
            .to_owned();
        draft.path = Some(stored.clone());
        if let Some(existing) = self.flow_id_by_path(&stored)? {
            self.update_flow(&existing, &draft)
        } else {
            self.create_flow(&draft)
        }
    }

    /// Escreve o JSON do fluxo e guarda o caminho na linha.
    ///
    /// # Errors
    ///
    /// Falha se o fluxo não existir ou se o disco recusar o arquivo.
    pub fn export_flow_file(&self, id: &str, path: &Path) -> Result<FlowDocument, DbError> {
        let document = self
            .list_flows()?
            .into_iter()
            .find(|flow| flow.id == id)
            .ok_or(DbError::Missing)?;
        let stored = path
            .to_str()
            .ok_or(DbError::Invalid("caminho inválido"))?
            .to_owned();
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serialize_flow_file(&document)?)?;
        self.update_flow(
            id,
            &FlowDraft {
                name: document.name,
                template_id: document.template_id,
                path: Some(stored),
                items: document.items,
            },
        )
    }

    fn flow_id_by_path(&self, path: &str) -> Result<Option<String>, DbError> {
        let connection = self.lock()?;
        let mut statement = connection.prepare("SELECT id FROM flows WHERE path = ?1")?;
        let mut rows = statement.query([path])?;
        match rows.next()? {
            Some(row) => Ok(Some(row.get(0)?)),
            None => Ok(None),
        }
    }

    fn write_flow(&self, document: &FlowDocument, insert: bool) -> Result<(), DbError> {
        let connection = self.lock()?;
        let transaction = connection.unchecked_transaction()?;
        let now = utc_timestamp(unix_now());
        if insert {
            transaction.execute(
                "INSERT INTO flows (id, name, template_id, path, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?5)",
                params![
                    document.id,
                    document.name,
                    document.template_id,
                    document.path,
                    now
                ],
            )?;
        } else {
            let changed = transaction.execute(
                "UPDATE flows
                 SET name = ?2, template_id = ?3, path = ?4, updated_at = ?5
                 WHERE id = ?1",
                params![
                    document.id,
                    document.name,
                    document.template_id,
                    document.path,
                    now
                ],
            )?;
            if changed == 0 {
                return Err(DbError::Missing);
            }
            transaction.execute("DELETE FROM flow_items WHERE flow_id = ?1", [&document.id])?;
        }
        for (position, item) in document.items.iter().enumerate() {
            let overrides = serde_json::to_string(&item.overrides)
                .map_err(|_| DbError::Invalid("sobrescrita inválida"))?;
            let mesh = mesh_value(item.mesh);
            let index =
                i64::try_from(position).map_err(|_| DbError::Invalid("posição inválida"))?;
            transaction.execute(
                "INSERT INTO flow_items (id, flow_id, position, prompt, count, mesh, overrides, template_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    item.id,
                    document.id,
                    index,
                    item.prompt,
                    item.count,
                    mesh,
                    overrides,
                    item.template_id
                ],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }
}

fn load_items(connection: &Connection) -> Result<HashMap<String, Vec<FlowItem>>, DbError> {
    let mut statement = connection.prepare(
        "SELECT id, flow_id, prompt, count, mesh, overrides, template_id
         FROM flow_items
         ORDER BY flow_id, position, id",
    )?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(1)?,
            row.get::<_, String>(0)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, Option<String>>(6)?,
        ))
    })?;
    let mut grouped: HashMap<String, Vec<FlowItem>> = HashMap::new();
    for row in rows {
        let (flow_id, id, prompt, count, mesh, overrides, template_id) = row?;
        grouped.entry(flow_id).or_default().push(FlowItem {
            id,
            prompt,
            count,
            mesh: parse_mesh(&mesh)?,
            template_id,
            overrides: parse_overrides(&overrides)?,
        });
    }
    Ok(grouped)
}

fn parse_flow_file(text: &str) -> Result<FlowDraft, DbError> {
    let file: FlowFile =
        serde_json::from_str(text).map_err(|_| DbError::Invalid("arquivo de fluxo inválido"))?;
    if file.version != FLOW_FILE_VERSION {
        return Err(DbError::Invalid("versão de fluxo desconhecida"));
    }
    Ok(FlowDraft {
        name: file.name,
        template_id: file.template_id.clone(),
        path: None,
        items: file
            .items
            .into_iter()
            .enumerate()
            .map(|(index, item)| FlowItem {
                id: item
                    .id
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| format!("item-{}", index + 1)),
                prompt: item.prompt,
                count: item.count,
                mesh: item.mesh,
                template_id: item.template_id.or_else(|| file.template_id.clone()),
                overrides: item.overrides,
            })
            .collect(),
    })
}

fn serialize_flow_file(document: &FlowDocument) -> Result<String, DbError> {
    let file = FlowFile {
        version: FLOW_FILE_VERSION,
        name: document.name.clone(),
        template_id: document.template_id.clone(),
        items: document
            .items
            .iter()
            .map(|item| FlowFileItem {
                id: Some(item.id.clone()),
                prompt: item.prompt.clone(),
                count: item.count,
                mesh: item.mesh,
                template_id: item.template_id.clone(),
                overrides: item.overrides.clone(),
            })
            .collect(),
    };
    let mut text = serde_json::to_string_pretty(&file)
        .map_err(|_| DbError::Invalid("arquivo de fluxo inválido"))?;
    text.push('\n');
    Ok(text)
}

fn checked(draft: &FlowDraft) -> Result<FlowDraft, DbError> {
    let name = draft.name.trim();
    if name.is_empty() {
        return Err(DbError::Invalid("nome vazio"));
    }
    let mut seen = HashSet::new();
    let mut items = Vec::with_capacity(draft.items.len());
    for item in &draft.items {
        let mut checked_item = checked_item(item)?;
        if checked_item.id.is_empty() {
            checked_item.id = new_id();
        }
        if !seen.insert(checked_item.id.clone()) {
            return Err(DbError::Invalid("item repetido"));
        }
        items.push(checked_item);
    }
    Ok(FlowDraft {
        name: name.to_owned(),
        template_id: draft
            .template_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        path: draft
            .path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        items,
    })
}

fn checked_item(item: &FlowItem) -> Result<FlowItem, DbError> {
    if !(1..=64).contains(&item.count) {
        return Err(DbError::Invalid("quantidade inválida"));
    }
    let overrides = checked_overrides(&item.overrides)?;
    Ok(FlowItem {
        id: item.id.trim().to_owned(),
        prompt: item.prompt.clone(),
        count: item.count,
        mesh: item.mesh,
        template_id: item
            .template_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        overrides,
    })
}

fn checked_overrides(overrides: &FlowOverrides) -> Result<FlowOverrides, DbError> {
    if let Some(resolution) = overrides.resolution
        && !matches!(resolution, 512 | 768 | 1024)
    {
        return Err(DbError::Invalid("resolução inválida"));
    }
    if let Some(aspect) = &overrides.aspect
        && !matches!(
            aspect.as_str(),
            "1:1" | "4:3" | "3:2" | "16:9" | "21:9" | "3:4" | "2:3" | "9:16" | "9:21"
        )
    {
        return Err(DbError::Invalid("proporção inválida"));
    }
    if let Some(steps) = overrides.steps
        && !matches!(steps, 1..=500)
    {
        return Err(DbError::Invalid("passos inválidos"));
    }
    if let Some(cfg) = overrides.cfg
        && (!cfg.is_finite() || !(0.0..=30.0).contains(&cfg))
    {
        return Err(DbError::Invalid("cfg inválido"));
    }
    if let Some(seed) = &overrides.seed
        && !seed.is_empty()
        && seed.parse::<i64>().is_err()
    {
        return Err(DbError::Invalid("seed inválida"));
    }
    Ok(overrides.clone())
}

fn parse_mesh(value: &str) -> Result<MeshMode, DbError> {
    match value {
        "none" => Ok(MeshMode::None),
        "all" => Ok(MeshMode::All),
        "later" => Ok(MeshMode::Later),
        _ => Err(DbError::Invalid("modo 3D inválido")),
    }
}

fn mesh_value(mode: MeshMode) -> &'static str {
    match mode {
        MeshMode::None => "none",
        MeshMode::All => "all",
        MeshMode::Later => "later",
    }
}

fn parse_overrides(text: &str) -> Result<FlowOverrides, DbError> {
    serde_json::from_str(text).map_err(|_| DbError::Invalid("sobrescrita inválida"))
}

fn new_id() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    format!("{}-{sequence}", unix_now())
}

#[cfg(test)]
mod tests;
