//! Comandos da biblioteca e dos arquivos de fluxo.

use std::path::PathBuf;

use tauri::State;

use crate::db::{Database, FlowDocument, FlowDraft};

/// Lista os fluxos gravados.
///
/// # Errors
///
/// Falha se o banco não puder ser lido.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn list_flows(database: State<'_, Database>) -> Result<Vec<FlowDocument>, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database.list_flows().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Cria um fluxo.
///
/// # Errors
///
/// Falha se os campos forem inválidos ou se o banco recusar a linha.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn create_flow(
    database: State<'_, Database>,
    draft: FlowDraft,
) -> Result<FlowDocument, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database
            .create_flow(&draft)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Atualiza nome, template, caminho e itens.
///
/// # Errors
///
/// Falha se o fluxo não existir ou se os campos forem inválidos.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn update_flow(
    database: State<'_, Database>,
    id: String,
    draft: FlowDraft,
) -> Result<FlowDocument, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database
            .update_flow(&id, &draft)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Exclui um fluxo.
///
/// # Errors
///
/// Falha se o fluxo não existir.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn delete_flow(database: State<'_, Database>, id: String) -> Result<(), String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database.delete_flow(&id).map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Abre um JSON de fluxo e grava no banco.
///
/// # Errors
///
/// Falha se o arquivo for inválido ou se o banco recusar a escrita.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn import_flow_file(
    database: State<'_, Database>,
    path: String,
) -> Result<FlowDocument, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database
            .import_flow_file(&PathBuf::from(path))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Grava o JSON do fluxo no caminho informado.
///
/// # Errors
///
/// Falha se o fluxo não existir ou se o disco recusar o arquivo.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn export_flow_file(
    database: State<'_, Database>,
    id: String,
    path: String,
) -> Result<FlowDocument, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database
            .export_flow_file(&id, &PathBuf::from(path))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}
