use std::path::{Path, PathBuf};

use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use voxel_models::Catalog;

use crate::paths::AppPaths;
use crate::setup::encode_base64;

const EDIT_EVENT: &str = "edit://image";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EditEvent {
    preview: String,
    path: String,
    transparent: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EditedImage {
    pub path: String,
    pub transparent: bool,
}

/// Fora da thread da janela: a imagem aberta vira PNG para a prévia.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn open_edit_image(app: AppHandle, path: String) -> Result<EditedImage, String> {
    tauri::async_runtime::spawn_blocking(move || open_image(&app, &path))
        .await
        .map_err(|error| error.to_string())?
}

/// Fora da thread da janela: o U²-Net em CPU grava o PNG transparente.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn remove_background(
    app: AppHandle,
    paths: State<'_, AppPaths>,
    source: String,
) -> Result<EditedImage, String> {
    let paths = paths.inner().clone();
    tauri::async_runtime::spawn_blocking(move || cut_image(&app, &paths, &source))
        .await
        .map_err(|error| error.to_string())?
}

fn open_image(app: &AppHandle, path: &str) -> Result<EditedImage, String> {
    let source = Path::new(path);
    let loaded = voxel_imaging::read_png(source).map_err(|error| error.to_string())?;
    emit_preview(
        app,
        source,
        encode_base64(&loaded.bytes),
        loaded.transparent,
    )
}

fn cut_image(app: &AppHandle, paths: &AppPaths, source: &str) -> Result<EditedImage, String> {
    let output = cut_file(paths, Path::new(source))?;
    let bytes = std::fs::read(&output).map_err(|error| error.to_string())?;
    emit_preview(app, &output, encode_base64(&bytes), true)
}

pub(crate) fn cut_file(paths: &AppPaths, source: &Path) -> Result<PathBuf, String> {
    let output = voxel_imaging::sibling_cutout(source);
    voxel_imaging::remove_background(&u2net_path(paths)?, source, &output)
        .map_err(|error| error.to_string())?;
    Ok(output)
}

fn emit_preview(
    app: &AppHandle,
    path: &Path,
    preview: String,
    transparent: bool,
) -> Result<EditedImage, String> {
    let path = path.display().to_string();
    app.emit(
        EDIT_EVENT,
        EditEvent {
            preview,
            path: path.clone(),
            transparent,
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(EditedImage { path, transparent })
}

fn u2net_path(paths: &AppPaths) -> Result<PathBuf, String> {
    let folder = paths.data.join("models");
    if let Ok(catalog) = Catalog::bundled()
        && let Some(model) = catalog.models.iter().find(|model| model.id == "u2net")
    {
        let installed = folder.join(&model.filename);
        if installed.is_file() {
            return Ok(installed);
        }
    }
    let fallback = folder.join("u2net.onnx");
    if fallback.is_file() {
        return Ok(fallback);
    }
    Err("modelo u2net ausente".to_owned())
}
