//! Comandos da biblioteca de templates.

use tauri::State;

use crate::db::{Database, PromptTemplate, TemplateDraft};

/// Lista os templates gravados.
///
/// # Errors
///
/// Falha se o banco não puder ser lido.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn list_templates(database: State<'_, Database>) -> Result<Vec<PromptTemplate>, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database.list_templates().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Cria um template.
///
/// # Errors
///
/// Falha se os campos forem inválidos ou se o banco recusar a linha.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn create_template(
    database: State<'_, Database>,
    draft: TemplateDraft,
) -> Result<PromptTemplate, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database
            .create_template(&draft)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Atualiza os campos de um template.
///
/// # Errors
///
/// Falha se o template não existir ou se os campos forem inválidos.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn update_template(
    database: State<'_, Database>,
    id: String,
    draft: TemplateDraft,
) -> Result<PromptTemplate, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database
            .update_template(&id, &draft)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Duplica um template com outro nome.
///
/// # Errors
///
/// Falha se a origem não existir ou se o nome for inválido.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn duplicate_template(
    database: State<'_, Database>,
    id: String,
    name: String,
) -> Result<PromptTemplate, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database
            .duplicate_template(&id, &name)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Renomeia um template.
///
/// # Errors
///
/// Falha se o template não existir ou se o nome vier vazio.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn rename_template(
    database: State<'_, Database>,
    id: String,
    name: String,
) -> Result<PromptTemplate, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database
            .rename_template(&id, &name)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Exclui um template.
///
/// # Errors
///
/// Falha se o template não existir.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn delete_template(database: State<'_, Database>, id: String) -> Result<(), String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database
            .delete_template(&id)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}
