//! Imagem para malha: recorte o fundo, solte a VRAM do 2D e chame o worker Python.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use voxel_hardware::PythonBackend;
use voxel_models::Catalog;
use voxel_protocol::{ImageToMeshRequest, LoadModelRequest, MeshBackend, ModelKind, WorkerProcess};

use crate::background::cut_file;
use crate::live_job::LiveJobs;
use crate::paths::AppPaths;
use crate::python_env::{Environment, Layout, process_env};
use crate::setup::encode_base64;

const MESH_MODEL: &str = "triposr";
const STARTUP: Duration = Duration::from_secs(30);

/// PNG sem fundo e os dois arquivos da malha.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeshFiles {
    pub image_path: String,
    pub glb_path: String,
    pub obj_path: String,
    pub preview: String,
}

/// Espera o ambiente, recorta o fundo e grava GLB e OBJ.
///
/// # Errors
///
/// Falha se a imagem, o `U²-Net`, o `TripoSR` ou o worker não estiverem disponíveis.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn mesh_from_image(
    app: AppHandle,
    paths: State<'_, AppPaths>,
    source: String,
) -> Result<MeshFiles, String> {
    let paths = paths.inner().clone();
    tauri::async_runtime::spawn_blocking(move || from_image(&app, &paths, &source))
        .await
        .map_err(|error| error.to_string())?
}

fn from_image(app: &AppHandle, paths: &AppPaths, source: &str) -> Result<MeshFiles, String> {
    let jobs = app.state::<LiveJobs>();
    let _session = jobs.enter();
    if jobs.stopped() {
        return Err("cancelado".to_owned());
    }
    app.state::<Environment>().wait_ready(app)?;
    let input = Path::new(source);
    if !input.is_file() {
        return Err("imagem ausente".to_owned());
    }
    let cutout = cut_file(paths, input)?;
    let preview = encode_base64(&std::fs::read(&cutout).map_err(|error| error.to_string())?);
    let (glb, obj) = render(app, paths, &cutout)?;
    Ok(MeshFiles {
        image_path: cutout.display().to_string(),
        glb_path: glb,
        obj_path: obj,
        preview,
    })
}

fn render(app: &AppHandle, paths: &AppPaths, cutout: &Path) -> Result<(String, String), String> {
    let jobs = app.state::<LiveJobs>();
    if jobs.stopped() {
        return Err("cancelado".to_owned());
    }
    let python = Layout::new(&paths.data, &paths.cache).python();
    if !python.is_file() {
        return Err("ambiente 3D ausente".to_owned());
    }
    let model = installed(paths, MESH_MODEL, "model.ckpt")?;
    let prepared = process_env(backend_of(app, paths), current_isa().as_deref());
    let mut pairs = prepared.set;
    let sources = pythonpath(&worker_src()?, triposr_src().as_deref());
    pairs.push(("PYTHONPATH".to_owned(), sources));
    pairs.push(("PYTHONUNBUFFERED".to_owned(), "1".to_owned()));
    let chosen = std::env::var("VOXEL_MESH_BACKEND").unwrap_or_else(|_| MESH_MODEL.to_owned());
    pairs.push(("VOXEL_MESH_BACKEND".to_owned(), chosen));
    let env: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect();
    let remove: Vec<&str> = prepared.remove.iter().map(String::as_str).collect();
    let worker = WorkerProcess::spawn_isolated(
        &python,
        "ok",
        &["-m", "voxel_worker"],
        &env,
        &remove,
        STARTUP,
        STARTUP,
    )
    .map_err(|error| error.to_string())?;
    worker
        .load_model(&LoadModelRequest {
            model_id: MESH_MODEL.to_owned(),
            model_path: model.display().to_string(),
            kind: ModelKind::Mesh,
        })
        .map_err(|error| error.to_string())?;
    let (glb, obj) = mesh_pair(cutout);
    let job_id = format!("mesh-{}", std::process::id());
    jobs.track(worker.port(), worker.token(), &job_id);
    if jobs.stopped() {
        return Err("cancelado".to_owned());
    }
    let generated = worker.image_to_mesh(&ImageToMeshRequest {
        job_id,
        image_path: cutout.display().to_string(),
        model_id: MESH_MODEL.to_owned(),
        output_path: glb.display().to_string(),
    });
    let _ = worker.unload();
    generated.map_err(|error| {
        let message = error.to_string();
        if jobs.stopped() || message.contains("cancelado") {
            "cancelado".to_owned()
        } else {
            message
        }
    })?;
    if jobs.stopped() {
        return Err("cancelado".to_owned());
    }
    if !glb.is_file() || !obj.is_file() {
        return Err("a malha não foi gravada".to_owned());
    }
    Ok((glb.display().to_string(), obj.display().to_string()))
}

fn backend_of(app: &AppHandle, paths: &AppPaths) -> PythonBackend {
    let stored = app
        .state::<Environment>()
        .status(paths)
        .backend
        .unwrap_or_default();
    PythonBackend::from_id(&stored).unwrap_or(PythonBackend::Cpu)
}

fn current_isa() -> Option<String> {
    voxel_hardware::detect()
        .gpus
        .into_iter()
        .find_map(|gpu| gpu.isa)
}

fn installed(paths: &AppPaths, id: &str, filename: &str) -> Result<PathBuf, String> {
    let folder = paths.data.join("models");
    if let Ok(catalog) = Catalog::bundled()
        && let Some(model) = catalog.models.iter().find(|model| model.id == id)
    {
        let path = folder.join(&model.filename);
        if path.is_file() {
            return Ok(path);
        }
    }
    let fallback = folder.join(filename);
    if fallback.is_file() {
        Ok(fallback)
    } else {
        Err(format!("modelo {id} ausente"))
    }
}

fn worker_src() -> Result<PathBuf, String> {
    if let Ok(value) = std::env::var("VOXEL_PYTHON_SRC") {
        let path = PathBuf::from(&value);
        if path.join("voxel_worker").is_dir() {
            return Ok(path);
        }
    }
    let main = find_from(
        &search_starts(),
        Path::new("workers/python/src/voxel_worker/__main__.py"),
    )
    .ok_or("worker Python ausente")?;
    main.parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or("worker Python ausente".to_owned())
}

fn triposr_src() -> Option<PathBuf> {
    if let Ok(value) = std::env::var("VOXEL_TRIPOSR_SRC") {
        let path = PathBuf::from(value);
        if path.join("tsr").is_dir() {
            return Some(path);
        }
    }
    if let Some(system) = find_from(
        &search_starts(),
        Path::new("workers/python/third_party/TripoSR/tsr/system.py"),
    ) {
        return system
            .parent()
            .and_then(Path::parent)
            .map(Path::to_path_buf);
    }
    let home = std::env::var("HOME").ok()?;
    let cached = PathBuf::from(home).join(".cache/voxel-pesquisa/src/TripoSR");
    cached.join("tsr").is_dir().then_some(cached)
}

fn pythonpath(src: &Path, triposr: Option<&Path>) -> String {
    match triposr {
        Some(extra) => format!("{}:{}", src.display(), extra.display()),
        None => src.display().to_string(),
    }
}

fn mesh_pair(cutout: &Path) -> (PathBuf, PathBuf) {
    (cutout.with_extension("glb"), cutout.with_extension("obj"))
}

fn search_starts() -> Vec<PathBuf> {
    let mut starts = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        starts.push(current);
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        starts.push(parent.to_path_buf());
    }
    starts
}

fn find_from(starts: &[PathBuf], relative: &Path) -> Option<PathBuf> {
    for start in starts {
        let mut dir = Some(start.as_path());
        for _ in 0..8 {
            let Some(current) = dir else { break };
            let candidate = current.join(relative);
            if candidate.is_file() {
                return Some(candidate);
            }
            dir = current.parent();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_malha_fica_ao_lado_do_recorte() {
        let cutout = PathBuf::from("/tmp/imagem_sem_fundo.png");
        let (glb, obj) = mesh_pair(&cutout);
        assert_eq!(glb, PathBuf::from("/tmp/imagem_sem_fundo.glb"));
        assert_eq!(obj, PathBuf::from("/tmp/imagem_sem_fundo.obj"));
    }

    #[test]
    fn o_codigo_do_modelo_entra_no_pythonpath() {
        let src = Path::new("/repo/workers/python/src");
        assert_eq!(pythonpath(src, None), "/repo/workers/python/src");
        assert_eq!(
            pythonpath(src, Some(Path::new("/cache/TripoSR"))),
            "/repo/workers/python/src:/cache/TripoSR"
        );
    }

    #[test]
    fn encontra_o_worker_subindo_as_pastas() {
        let root = std::env::temp_dir().join(format!("voxel-mesh-{}", std::process::id()));
        let file = root.join("workers/python/src/voxel_worker/__main__.py");
        std::fs::create_dir_all(file.parent().expect("pasta")).expect("criar");
        std::fs::write(&file, "").expect("arquivo");
        let nested = root.join("apps/desktop");
        let found = find_from(
            &[nested],
            Path::new("workers/python/src/voxel_worker/__main__.py"),
        );
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(found, Some(file));
    }
}
