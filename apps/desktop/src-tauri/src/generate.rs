use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use voxel_protocol::{
    ImageBackend, LoadModelRequest, ModelKind, Txt2ImgRequest, WorkerEvent, WorkerProcess,
};

use crate::db::Database;
use crate::paths::AppPaths;
use crate::probe::{SdBinaries, discover};
use crate::setup::encode_base64;

const BACKENDS: [&str; 4] = ["vulkan", "rocm", "cuda", "cpu"];

/// Escolha mostrada nas configurações.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageBackendChoice {
    pub current: String,
    pub recommended: Option<String>,
}

/// Pedido de geração vindo da interface.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateRequest {
    pub prompt: String,
    pub negative_prompt: Option<String>,
}

/// Imagem pronta, com a última prévia em base64.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedImage {
    pub path: String,
    pub preview: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationProgress {
    step: i64,
    total: i64,
    preview: Option<String>,
}

/// Backend em uso: a escolha manual, senão o recomendado pelo setup.
///
/// # Errors
///
/// Falha se o banco não puder ser lido.
pub fn backend_choice(database: &Database) -> Result<ImageBackendChoice, String> {
    let recommended = database
        .setup_flag("recommended")
        .map_err(|error| error.to_string())?
        .or(database
            .setup_flag("backend")
            .map_err(|error| error.to_string())?);
    let current = database
        .image_backend_override()
        .map_err(|error| error.to_string())?
        .filter(|value| BACKENDS.contains(&value.as_str()))
        .or_else(|| recommended.clone())
        .unwrap_or_else(|| "vulkan".to_owned());
    Ok(ImageBackendChoice {
        current,
        recommended,
    })
}

/// Grava a escolha manual.
///
/// # Errors
///
/// Falha se o backend não existir ou o banco não puder ser gravado.
pub fn set_backend(database: &Database, backend: &str) -> Result<ImageBackendChoice, String> {
    if !BACKENDS.contains(&backend) {
        return Err("backend desconhecido".to_owned());
    }
    database
        .set_image_backend_override(Some(backend))
        .map_err(|error| error.to_string())?;
    backend_choice(database)
}

/// Apaga a escolha manual e volta ao backend do setup.
///
/// # Errors
///
/// Falha se o banco não puder ser gravado.
pub fn restore_backend(database: &Database) -> Result<ImageBackendChoice, String> {
    database
        .set_image_backend_override(None)
        .map_err(|error| error.to_string())?;
    backend_choice(database)
}

/// Roda um txt2img no worker e publica passo e prévia.
///
/// # Errors
///
/// Falha se o modelo, o binário ou o worker não estiverem disponíveis.
pub fn generate(
    app: &AppHandle,
    database: &Database,
    paths: &AppPaths,
    request: &GenerateRequest,
) -> Result<GeneratedImage, String> {
    let choice = backend_choice(database)?;
    let bins = discover();
    let model = bins.model.clone().ok_or("modelo sd15 ausente")?;
    let cli = binary_for(&bins, &choice.current).ok_or("sd-cli ausente")?;
    let cli_path = cli.display().to_string();
    let worker_bin = worker_binary()?;
    let worker = WorkerProcess::spawn_with(
        &worker_bin,
        "ok",
        &[
            ("VOXEL_SD_CLI", cli_path.as_str()),
            ("VOXEL_IMAGE_BACKEND", choice.current.as_str()),
        ],
        Duration::from_secs(180),
    )
    .map_err(|error| error.to_string())?;
    worker
        .load_model(&LoadModelRequest {
            model_id: "sd15".to_owned(),
            model_path: model.display().to_string(),
            kind: ModelKind::Image,
        })
        .map_err(|error| error.to_string())?;
    let directory = paths.cache.join("generations");
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let job_id = format!("gen-{}", std::process::id());
    let output = directory.join(format!("{job_id}.png"));
    let port = worker.port();
    let token = worker.token().to_owned();
    let watched = job_id.clone();
    let emitter = app.clone();
    let progress = std::thread::spawn(move || {
        let _ = WorkerProcess::watch(port, &token, &watched, |event| {
            if let WorkerEvent::Progress(step) = event {
                let preview = step
                    .preview_path
                    .as_deref()
                    .and_then(|path| preview_base64(Path::new(path)));
                let _ = emitter.emit(
                    "generation://progress",
                    GenerationProgress {
                        step: step.step,
                        total: step.total,
                        preview,
                    },
                );
            }
        });
    });
    let generated = worker
        .txt2img(&Txt2ImgRequest {
            job_id,
            prompt: request.prompt.clone(),
            negative_prompt: request.negative_prompt.clone(),
            model_id: "sd15".to_owned(),
            output_path: output.display().to_string(),
            steps: 4,
            width: 512,
            height: 512,
            seed: 1,
        })
        .map_err(|error| error.to_string())?;
    let _ = progress.join();
    Ok(GeneratedImage {
        path: generated.path,
        preview: preview_base64(&output),
    })
}

fn binary_for(bins: &SdBinaries, backend: &str) -> Option<PathBuf> {
    match backend {
        "rocm" => bins.rocm.clone(),
        "cuda" => bins.cuda.clone(),
        "cpu" => bins.cpu.clone(),
        _ => bins.vulkan.clone(),
    }
}

fn worker_binary() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("VOXEL_SDCPP_WORKER") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }
    }
    let mut starts = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        starts.push(current);
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        starts.push(parent.to_path_buf());
    }
    for start in starts {
        let mut dir = Some(start.as_path());
        for _ in 0..8 {
            let Some(current) = dir else { break };
            for relative in ["target/debug/voxel-sdcpp", "target/release/voxel-sdcpp"] {
                let candidate = current.join(relative);
                if candidate.is_file() {
                    return Ok(candidate);
                }
            }
            dir = current.parent();
        }
    }
    Err("worker voxel-sdcpp ausente".to_owned())
}

fn preview_base64(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    bytes
        .starts_with(b"\x89PNG\r\n\x1a\n")
        .then(|| encode_base64(&bytes))
}

#[cfg(test)]
mod tests {
    use super::{backend_choice, restore_backend, set_backend};
    use crate::db::Database;

    #[test]
    fn a_escolha_manual_vence_e_a_restauracao_volta_ao_setup() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        database.set_setup_flag("recommended", "vulkan").unwrap();
        assert_eq!(backend_choice(&database).unwrap().current, "vulkan");
        let chosen = set_backend(&database, "cpu").unwrap();
        assert_eq!(chosen.current, "cpu");
        assert_eq!(chosen.recommended.as_deref(), Some("vulkan"));
        assert!(set_backend(&database, "metal").is_err());
        assert_eq!(restore_backend(&database).unwrap().current, "vulkan");
    }
}
