use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use voxel_hardware::{GenerationDefaults, Profile, defaults_for};
use voxel_models::Catalog;
use voxel_protocol::{
    ImageBackend, LoadModelRequest, ModelKind, Txt2ImgRequest, WorkerEvent, WorkerProcess,
};

use crate::db::{Database, GenerationRecord};
use crate::output::{self, ImageMetadata};
use crate::paths::AppPaths;
use crate::probe::{SdBinaries, discover};
use crate::setup::encode_base64;

const BACKENDS: [&str; 4] = ["vulkan", "rocm", "cuda", "cpu"];
const SIDE_MIN: u32 = 256;
const SIDE_MAX: u32 = 1024;
const SIDE_STEP: u32 = 64;

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
    pub model_id: String,
    pub width: u32,
    pub height: u32,
    pub steps: u32,
    pub cfg: f64,
    pub seed: Option<i64>,
    pub count: u32,
}

/// Imagem pronta e a seed usada. A interface lê o arquivo pelo protocolo de assets.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedImage {
    pub path: String,
    pub seed: i64,
    pub model_id: String,
    pub images: Vec<GeneratedFile>,
}

/// Uma imagem já gravada nesta geração.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GeneratedFile {
    pub path: String,
    pub seed: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct GenerationProgress {
    step: i64,
    total: i64,
    preview: Option<String>,
    index: u32,
    count: u32,
    ready: bool,
    path: Option<String>,
    seed: Option<i64>,
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

/// Parâmetros iniciais do painel, lidos do perfil gravado no setup.
#[must_use]
pub fn generation_defaults(database: &Database) -> GenerationDefaults {
    let profile = database
        .setup_flag("profile")
        .ok()
        .flatten()
        .as_deref()
        .and_then(Profile::from_id)
        .unwrap_or(Profile::Intermediario);
    defaults_for(profile)
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
    let model = resolve_model(paths, &request.model_id, &bins)?;
    let cli = binary_for(&bins, &choice.current).ok_or("sd-cli ausente")?;
    let limits = limits_of(request);
    let cli_path = cli.display().to_string();
    let worker = WorkerProcess::spawn_with(
        &worker_binary()?,
        "ok",
        &[
            ("VOXEL_SD_CLI", cli_path.as_str()),
            ("VOXEL_IMAGE_BACKEND", choice.current.as_str()),
        ],
        // Saúde e carga do modelo. A imagem em si espera bem mais, senão 1024 px cai no meio.
        Duration::from_mins(3),
    )
    .map_err(|error| error.to_string())?;
    worker
        .load_model(&LoadModelRequest {
            model_id: request.model_id.clone(),
            model_path: model.display().to_string(),
            kind: ModelKind::Image,
        })
        .map_err(|error| error.to_string())?;
    let folder = output::item_directory(&output::today_dir(&output_root()), &request.prompt);
    std::fs::create_dir_all(&folder).map_err(|error| error.to_string())?;
    let seed = request.seed.unwrap_or_else(random_seed);
    let mut last_path = String::new();
    let mut saved = Vec::new();
    let mut images = Vec::new();
    for index in 0..limits.count {
        let file = format!("imagem_{:02}.png", index + 1);
        let output = folder.join(&file);
        let image_seed = seed.saturating_add(i64::from(index));
        let started = Instant::now();
        render_one(app, &worker, request, &limits, &output, image_seed, index)?;
        let metadata = metadata_for(
            request,
            &choice.current,
            &limits,
            image_seed,
            &file,
            started,
        );
        output::embed_metadata(&output, &metadata)?;
        database
            .record_generation(&record_of(&metadata, &output))
            .map_err(|error| error.to_string())?;
        let path = output.display().to_string();
        let _ = app.emit(
            "generation://progress",
            GenerationProgress {
                step: limits.steps,
                total: limits.steps,
                preview: preview_base64(&output),
                index,
                count: limits.count,
                ready: true,
                path: Some(path.clone()),
                seed: Some(image_seed),
            },
        );
        last_path.clone_from(&path);
        images.push(GeneratedFile {
            path,
            seed: image_seed,
        });
        saved.push(metadata);
    }
    output::write_metadata(&folder, &saved)?;
    Ok(GeneratedImage {
        path: last_path,
        seed,
        model_id: request.model_id.clone(),
        images,
    })
}

struct Limits {
    width: i64,
    height: i64,
    steps: i64,
    cfg: f64,
    count: u32,
}

fn snap_side(value: u32) -> i64 {
    let clamped = value.clamp(SIDE_MIN, SIDE_MAX);
    let rounded = clamped.saturating_add(SIDE_STEP / 2) / SIDE_STEP * SIDE_STEP;
    i64::from(rounded.clamp(SIDE_MIN, SIDE_MAX))
}

fn limits_of(request: &GenerateRequest) -> Limits {
    Limits {
        width: snap_side(request.width),
        height: snap_side(request.height),
        steps: i64::from(request.steps.clamp(1, 500)),
        cfg: if request.cfg.is_finite() {
            request.cfg.clamp(0.0, 30.0)
        } else {
            7.0
        },
        count: request.count.clamp(1, 4),
    }
}

fn render_one(
    app: &AppHandle,
    worker: &WorkerProcess,
    request: &GenerateRequest,
    limits: &Limits,
    output: &Path,
    seed: i64,
    index: u32,
) -> Result<(), String> {
    let job_id = format!("gen-{}-{index}", std::process::id());
    let port = worker.port();
    let token = worker.token().to_owned();
    let watched = job_id.clone();
    let emitter = app.clone();
    let expected = limits.steps;
    let image_count = limits.count;
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    let progress = std::thread::spawn(move || {
        let _ = WorkerProcess::watch_until(port, &token, &watched, &flag, |event| {
            if let WorkerEvent::Progress(step) = event {
                // O decode em tiles também imprime passo/total, e não é o denoising.
                if step.total != expected {
                    return;
                }
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
                        index,
                        count: image_count,
                        ready: false,
                        path: None,
                        seed: None,
                    },
                );
            }
        });
    });
    let generated = worker.txt2img(&Txt2ImgRequest {
        job_id,
        prompt: request.prompt.clone(),
        negative_prompt: request.negative_prompt.clone(),
        model_id: request.model_id.clone(),
        output_path: output.display().to_string(),
        steps: limits.steps,
        width: limits.width,
        height: limits.height,
        seed,
        cfg_scale: Some(limits.cfg),
    });
    stop.store(true, Ordering::Relaxed);
    let _ = progress.join();
    generated.map_err(|error| error.to_string())?;
    Ok(())
}

fn metadata_for(
    request: &GenerateRequest,
    backend: &str,
    limits: &Limits,
    seed: i64,
    file: &str,
    started: Instant,
) -> ImageMetadata {
    ImageMetadata {
        prompt: request.prompt.clone(),
        negative_prompt: request.negative_prompt.clone(),
        model_id: request.model_id.clone(),
        backend: backend.to_owned(),
        seed,
        steps: limits.steps,
        width: limits.width,
        height: limits.height,
        cfg: limits.cfg,
        elapsed_ms: i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX),
        created_at: output::utc_timestamp(output::unix_now()),
        file: file.to_owned(),
    }
}

fn record_of(metadata: &ImageMetadata, output: &Path) -> GenerationRecord {
    GenerationRecord {
        path: output.display().to_string(),
        prompt: metadata.prompt.clone(),
        negative_prompt: metadata.negative_prompt.clone(),
        model_id: metadata.model_id.clone(),
        backend: metadata.backend.clone(),
        seed: metadata.seed,
        steps: metadata.steps,
        width: metadata.width,
        height: metadata.height,
        cfg: metadata.cfg,
        elapsed_ms: metadata.elapsed_ms,
        created_at: metadata.created_at.clone(),
    }
}

fn resolve_model(paths: &AppPaths, model_id: &str, bins: &SdBinaries) -> Result<PathBuf, String> {
    let folder = paths.data.join("models");
    if let Ok(catalog) = Catalog::bundled()
        && let Some(model) = catalog.models.iter().find(|model| model.id == model_id)
    {
        let installed = folder.join(&model.filename);
        if installed.is_file() {
            return Ok(installed);
        }
    }
    if model_id == "sd15"
        && let Some(path) = &bins.model
    {
        return Ok(path.clone());
    }
    let imported = folder.join(model_id);
    if imported.is_file() {
        return Ok(imported);
    }
    Err("modelo ausente".to_owned())
}

fn output_root() -> PathBuf {
    let home = std::env::var_os("HOME").map_or_else(|| PathBuf::from("."), PathBuf::from);
    let text = std::fs::read_to_string(home.join(".config/user-dirs.dirs")).unwrap_or_default();
    output::pictures_root(&home, &text)
}

fn random_seed() -> i64 {
    let mut bytes = [0_u8; 8];
    let opened = std::fs::File::open("/dev/urandom").and_then(|mut file| {
        use std::io::Read;
        file.read_exact(&mut bytes)
    });
    if opened.is_err() {
        return 1;
    }
    i64::from_le_bytes(bytes) & 0x7fff_ffff
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

/// PNG em base64 para o `<img>`. No Linux o `asset://` não carrega dentro da imagem.
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

    #[test]
    fn o_perfil_define_os_passos_e_o_modelo_resolvido_existe() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        database.set_setup_flag("profile", "minimo").unwrap();
        assert_eq!(super::generation_defaults(&database).steps, 20);
        database.set_setup_flag("profile", "intermediario").unwrap();
        assert_eq!(super::generation_defaults(&database).steps, 25);

        let root = std::env::temp_dir().join(format!("voxel-model-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let data = root.join("data");
        let models = data.join("models");
        std::fs::create_dir_all(&models).unwrap();
        let file = models.join("local.safetensors");
        std::fs::write(&file, "peso").unwrap();
        let paths = crate::paths::AppPaths {
            config: root.join("config"),
            data,
            state: root.join("state"),
            cache: root.join("cache"),
        };
        let bins = crate::probe::SdBinaries {
            vulkan: None,
            rocm: None,
            cuda: None,
            cpu: None,
            model: None,
        };
        let resolved = super::resolve_model(&paths, "local.safetensors", &bins).unwrap();
        assert_eq!(resolved, file);
        assert!(super::resolve_model(&paths, "ausente", &bins).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_proporcao_entra_na_grade_de_64() {
        let mut request = super::GenerateRequest {
            prompt: "teste".to_owned(),
            negative_prompt: None,
            model_id: "sd15".to_owned(),
            width: 1024,
            height: 576,
            steps: 20,
            cfg: 7.0,
            seed: None,
            count: 1,
        };
        let wide = super::limits_of(&request);
        assert_eq!((wide.width, wide.height), (1024, 576));
        request.width = 576;
        request.height = 1024;
        let tall = super::limits_of(&request);
        assert_eq!((tall.width, tall.height), (576, 1024));
        request.width = 288;
        request.height = 4_000;
        let snapped = super::limits_of(&request);
        assert_eq!((snapped.width, snapped.height), (320, 1024));
    }
}
