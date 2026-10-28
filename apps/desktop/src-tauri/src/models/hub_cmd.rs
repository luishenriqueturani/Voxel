//! Comandos da busca no Hugging Face.

use std::time::{Duration, Instant};

use serde::Deserialize;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use voxel_models::{HubFile, HubPackage, fetch, find_file, hub_job, search};

use super::ListedModel;
use super::package_store::{self, PackageSpec};
use crate::db::Database;
use crate::paths::AppPaths;

const API: &str = "https://huggingface.co";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    model_id: String,
    received: u64,
    total: u64,
    phase: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HubChoice {
    pub repo: String,
    pub filename: String,
    #[serde(default)]
    pub package_name: String,
    #[serde(default)]
    pub opener: String,
    #[serde(default)]
    pub directory: bool,
    #[serde(default)]
    pub marker: String,
    #[serde(default)]
    pub parts: Vec<String>,
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
) -> Result<Vec<HubPackage>, String> {
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

/// Baixa uma parte, confere o hash quando o Hub informa e grava o marcador no fim.
///
/// # Errors
///
/// Falha se o arquivo, o token ou a rede não permitirem concluir.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn download_hub_model(
    app: AppHandle,
    database: tauri::State<'_, Database>,
    paths: tauri::State<'_, AppPaths>,
    choice: HubChoice,
) -> Result<ListedModel, String> {
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
) -> Result<ListedModel, String> {
    let token = database.hf_token().map_err(|error| error.to_string())?;
    let file = find_file(API, &choice.repo, &choice.filename, token.as_deref()).map_err(explain)?;
    let job = hub_job(&file, token.as_deref()).map_err(explain)?;
    let dir = paths.data.join("models");
    let destination = package_store::hub_part_path(&dir, &file.repo, &file.filename)?;
    if !destination.is_file() {
        let id = format!("{}/{}", file.repo, file.filename);
        let mut emitted: Option<Instant> = None;
        fetch(&job, &destination, |phase, received, total| {
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
                    phase: match phase {
                        voxel_models::Transfer::Download => "download",
                        voxel_models::Transfer::Verify => "verify",
                    },
                },
            );
        })
        .map_err(explain)?;
    }
    let stored = package_store::complete_package(&dir, &file.repo, &spec_from(choice, &file))?;
    Ok(listed_download(&file, stored.as_ref()))
}

fn spec_from(choice: &HubChoice, file: &HubFile) -> PackageSpec {
    let mut parts = choice.parts.clone();
    if parts.is_empty() || !parts.iter().any(|part| part == &file.filename) {
        parts.push(file.filename.clone());
    }
    let name = if choice.package_name.is_empty() {
        file.filename.clone()
    } else {
        choice.package_name.clone()
    };
    let opener = if choice.directory || !choice.opener.is_empty() {
        choice.opener.clone()
    } else {
        file.filename.clone()
    };
    PackageSpec {
        name,
        opener,
        directory: choice.directory,
        license: file.license.clone(),
        parts,
        marker: choice.marker.clone(),
    }
}

fn listed_download(file: &HubFile, stored: Option<&package_store::StoredPackage>) -> ListedModel {
    let Some(stored) = stored else {
        return ListedModel {
            id: file.filename.clone(),
            name: file.filename.clone(),
            license: file.license.clone(),
            bytes: file.bytes,
            filename: file.filename.clone(),
            installed: false,
            catalog: false,
            compatibility: "compatible",
            reason: None,
            architecture: None,
            kind: "image".to_owned(),
        };
    };
    ListedModel {
        id: stored.open.clone(),
        name: stored.name.clone(),
        license: stored.license.clone(),
        bytes: stored.bytes,
        filename: stored.open.clone(),
        installed: true,
        catalog: false,
        compatibility: "compatible",
        reason: None,
        architecture: None,
        kind: "image".to_owned(),
    }
}

fn explain(error: voxel_models::ModelError) -> String {
    match error {
        voxel_models::ModelError::Invalid(detail) => detail,
        voxel_models::ModelError::TokenRequired => "token ausente".to_owned(),
        other => other.to_string(),
    }
}
