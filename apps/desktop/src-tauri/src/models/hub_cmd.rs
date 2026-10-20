//! Comandos da busca no Hugging Face.

use std::time::{Duration, Instant};

use serde::Deserialize;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use voxel_models::{Catalog, HubFile, disk_name, fetch, find_file, hub_job, search};

use super::{current_facts, describe_downloaded};
use crate::db::Database;
use crate::paths::AppPaths;

const API: &str = "https://huggingface.co";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    model_id: String,
    received: u64,
    total: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubChoice {
    pub repo: String,
    pub filename: String,
}

/// Procura pesos no Hub. A busca vazia não abre a rede.
///
/// # Errors
///
/// Falha se o token não puder ser lido ou se o Hub recusar a busca.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn search_hub_models(
    database: tauri::State<'_, Database>,
    query: String,
) -> Result<Vec<HubFile>, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        if query.trim().is_empty() {
            return Ok(Vec::new());
        }
        let token = database.hf_token().map_err(|error| error.to_string())?;
        search(&query, token.as_deref()).map_err(explain)
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Baixa o arquivo escolhido, confere o hash quando o Hub informa e lê o cabeçalho.
///
/// # Errors
///
/// Falha se o arquivo, o token, a rede ou o cabeçalho não permitirem concluir.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn download_hub_model(
    app: AppHandle,
    database: tauri::State<'_, Database>,
    paths: tauri::State<'_, AppPaths>,
    choice: HubChoice,
) -> Result<super::ListedModel, String> {
    let database = database.inner().clone();
    let paths = paths.inner().clone();
    tauri::async_runtime::spawn_blocking(move || download(&app, &database, &paths, &choice))
        .await
        .map_err(|error| error.to_string())?
}

fn download(
    app: &AppHandle,
    database: &Database,
    paths: &AppPaths,
    choice: &HubChoice,
) -> Result<super::ListedModel, String> {
    let token = database.hf_token().map_err(|error| error.to_string())?;
    let file = find_file(API, &choice.repo, &choice.filename, token.as_deref()).map_err(explain)?;
    let job = hub_job(&file, token.as_deref()).map_err(explain)?;
    let saved = disk_name(&file.filename);
    if Catalog::bundled()
        .map_err(|error| error.to_string())?
        .models
        .iter()
        .any(|model| model.filename == saved)
    {
        return Err("arquivo já existe".to_owned());
    }
    let dir = paths.data.join("models");
    let destination = dir.join(&saved);
    if !destination.is_file() {
        let id = format!("{}/{}", file.repo, file.filename);
        let mut emitted: Option<Instant> = None;
        fetch(&job, &destination, |received, total| {
            let finished = total > 0 && received >= total;
            let recent =
                emitted.is_some_and(|instant| instant.elapsed() < Duration::from_millis(200));
            if !finished && recent {
                return;
            }
            emitted = Some(Instant::now());
            let _ = app.emit(
                "model://progress",
                DownloadProgress {
                    model_id: id.clone(),
                    received,
                    total,
                },
            );
        })
        .map_err(explain)?;
    }
    describe_downloaded(&dir, &saved, &file.license, &current_facts())
}

fn explain(error: voxel_models::ModelError) -> String {
    match error {
        voxel_models::ModelError::Invalid(detail) => detail,
        voxel_models::ModelError::TokenRequired => "token ausente".to_owned(),
        other => other.to_string(),
    }
}
