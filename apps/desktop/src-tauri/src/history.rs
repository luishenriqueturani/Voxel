//! Comandos da lista de prompts e da galeria.

use serde::Serialize;
use tauri::State;

use crate::db::{Database, GalleryRow, PromptEntry, PromptImage};
use crate::setup::encode_base64;

const THUMB_EDGE: u32 = 192;
const GALLERY_LIMIT: i64 = 48;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryItem {
    pub path: String,
    pub prompt: String,
    pub negative_prompt: String,
    pub model_id: String,
    pub steps: i64,
    pub width: i64,
    pub height: i64,
    pub cfg: f64,
    pub seed: i64,
    pub created_at: String,
    pub preview: String,
}

/// Prompts recentes, com os fixados no topo.
///
/// # Errors
///
/// Falha se o banco não puder ser lido.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn list_recent_prompts(
    database: State<'_, Database>,
) -> Result<Vec<PromptEntry>, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database.recent_prompts().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Fixa ou solta um prompt da lista.
///
/// # Errors
///
/// Falha se o banco recusar a escrita.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn set_prompt_favorite(
    database: State<'_, Database>,
    entry: PromptEntry,
    pinned: bool,
) -> Result<(), String> {
    database
        .set_favorite(&entry, pinned)
        .map_err(|error| error.to_string())
}

/// Imagens da geração mais recente do prompt que ainda estão no disco.
///
/// # Errors
///
/// Falha se o banco não puder ser lido.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn list_prompt_images(
    database: State<'_, Database>,
    entry: PromptEntry,
) -> Result<Vec<PromptImage>, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        database
            .prompt_images(&entry)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

/// Gerações mais novas, com uma miniatura JPEG.
///
/// # Errors
///
/// Falha se o banco não puder ser lido. Uma imagem ausente fica sem miniatura.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn list_gallery(database: State<'_, Database>) -> Result<Vec<GalleryItem>, String> {
    let database = database.inner().clone();
    tauri::async_runtime::spawn_blocking(move || gallery_of(&database))
        .await
        .map_err(|error| error.to_string())?
}

fn gallery_of(database: &Database) -> Result<Vec<GalleryItem>, String> {
    database
        .gallery(GALLERY_LIMIT)
        .map_err(|error| error.to_string())
        .map(|rows| rows.into_iter().map(with_preview).collect())
}

fn with_preview(row: GalleryRow) -> GalleryItem {
    GalleryItem {
        preview: thumbnail(&row.path),
        path: row.path,
        prompt: row.prompt,
        negative_prompt: row.negative_prompt,
        model_id: row.model_id,
        steps: row.steps,
        width: row.width,
        height: row.height,
        cfg: row.cfg,
        seed: row.seed,
        created_at: row.created_at,
    }
}

fn thumbnail(path: &str) -> String {
    let Ok(bytes) = voxel_imaging::thumbnail_jpeg(std::path::Path::new(path), THUMB_EDGE) else {
        return String::new();
    };
    if bytes.is_empty() {
        return String::new();
    }
    encode_base64(&bytes)
}
