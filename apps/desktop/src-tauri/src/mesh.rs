//! Imagem para malha: recorte o fundo, solte a VRAM do 2D e chame o worker Python.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimized_glb: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimized_obj: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimized_fbx: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
}

/// Pedido da cópia otimizada. Ausente quando a opção está desligada.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MeshOptimize {
    pub triangles: u32,
    pub texture_size: u32,
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
    views: Vec<String>,
    primary: String,
    optimize: Option<MeshOptimize>,
) -> Result<MeshFiles, String> {
    let paths = paths.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        from_image(&app, &paths, &views, &primary, optimize)
    })
    .await
    .map_err(|error| error.to_string())?
}

pub(crate) fn from_image(
    app: &AppHandle,
    paths: &AppPaths,
    views: &[String],
    primary: &str,
    optimize: Option<MeshOptimize>,
) -> Result<MeshFiles, String> {
    let jobs = app.state::<LiveJobs>();
    let _session = jobs.enter();
    if jobs.stopped() {
        return Err("cancelado".to_owned());
    }
    app.state::<Environment>().wait_ready(app)?;
    let ordered: Vec<PathBuf> = views.iter().map(PathBuf::from).collect();
    let primary_path = PathBuf::from(primary);
    if ordered.is_empty() || !ordered.iter().any(|path| path == &primary_path) {
        return Err("vista principal ausente".to_owned());
    }
    for path in &ordered {
        if !path.is_file() {
            return Err("imagem ausente".to_owned());
        }
    }
    let model = std::env::var("VOXEL_MESH_BACKEND").unwrap_or_else(|_| MESH_MODEL.to_owned());
    let inputs = views_for_model(measured_multiview_models(), &model, &ordered, &primary_path);
    let source = inputs.first().ok_or("vista principal ausente")?;
    let cutout = cut_file(paths, source)?;
    let preview = encode_base64(&std::fs::read(&cutout).map_err(|error| error.to_string())?);
    let (glb, obj) = render(app, paths, &cutout)?;
    let folder = Path::new(&glb)
        .parent()
        .ok_or("imagem ausente")?
        .to_path_buf();
    let references = copy_references(&folder, &ordered, &primary_path)?;
    let optimized = optimized_copy(Path::new(&glb), optimize)?;
    Ok(MeshFiles {
        image_path: cutout.display().to_string(),
        glb_path: glb,
        obj_path: obj,
        preview,
        optimized_glb: optimized.as_ref().map(|item| item.0.clone()),
        optimized_obj: optimized.as_ref().map(|item| item.1.clone()),
        optimized_fbx: optimized.as_ref().map(|item| item.2.clone()),
        references,
    })
}

/// Nenhum modelo multi-vista foi medido nesta máquina. O `InstantMesh` não sobe:
/// o ponto de entrada pede um rasterizador CUDA.
fn measured_multiview_models() -> &'static [&'static str] {
    &[]
}

fn views_for_model(
    measured: &[&str],
    model: &str,
    ordered: &[PathBuf],
    primary: &Path,
) -> Vec<PathBuf> {
    if measured.contains(&model) {
        ordered.to_vec()
    } else {
        vec![primary.to_path_buf()]
    }
}

fn copy_references(dir: &Path, views: &[PathBuf], primary: &Path) -> Result<Vec<String>, String> {
    let mut written = Vec::new();
    for view in views {
        if view.as_path() == primary {
            continue;
        }
        written.push(unique_copy(dir, view)?.display().to_string());
    }
    Ok(written)
}

fn unique_copy(dir: &Path, source: &Path) -> Result<PathBuf, String> {
    let name = source.file_name().ok_or("imagem ausente")?;
    let direct = dir.join(name);
    if direct.as_path() == source {
        return Ok(direct);
    }
    if !direct.exists() {
        std::fs::copy(source, &direct).map_err(|error| error.to_string())?;
        return Ok(direct);
    }
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or("imagem ausente")?;
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("png");
    for index in 2..1_000 {
        let candidate = dir.join(format!("{stem}_{index}.{extension}"));
        if !candidate.exists() {
            std::fs::copy(source, &candidate).map_err(|error| error.to_string())?;
            return Ok(candidate);
        }
    }
    Err("imagem ausente".to_owned())
}

fn optimized_copy(
    glb: &Path,
    options: Option<MeshOptimize>,
) -> Result<Option<(String, String, String)>, String> {
    let Some(options) = options else {
        return Ok(None);
    };
    let written = voxel_mesh::optimize(
        glb,
        voxel_mesh::Options {
            triangles: options.triangles,
            texture_size: options.texture_size,
        },
    )
    .map_err(|error| error.to_string())?;
    Ok(Some((
        written.glb.display().to_string(),
        written.obj.display().to_string(),
        written.fbx.display().to_string(),
    )))
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

    #[test]
    fn o_triposr_usa_a_principal_e_o_multi_vista_usa_a_lista() {
        let ordered = vec![
            PathBuf::from("/tmp/frente.png"),
            PathBuf::from("/tmp/lado.png"),
        ];
        let primary = PathBuf::from("/tmp/lado.png");
        assert_eq!(
            views_for_model(&[], "triposr", &ordered, &primary),
            vec![primary.clone()]
        );
        assert_eq!(
            views_for_model(&["instantmesh"], "instantmesh", &ordered, &primary),
            ordered
        );
    }

    #[test]
    fn as_outras_vistas_vao_para_a_pasta_do_resultado() {
        let root = std::env::temp_dir().join(format!("voxel-vistas-{}", std::process::id()));
        let folder = root.join("saida");
        std::fs::create_dir_all(&folder).expect("pasta");
        let primary = folder.join("frente.png");
        let side = root.join("lado.png");
        std::fs::write(&primary, "frente").expect("frente");
        std::fs::write(&side, "lado").expect("lado");
        let copied =
            copy_references(&folder, &[primary.clone(), side.clone()], &primary).expect("copiar");
        assert_eq!(copied, vec![folder.join("lado.png").display().to_string()]);
        assert_eq!(
            std::fs::read(folder.join("lado.png")).expect("ler"),
            b"lado"
        );
        let again = copy_references(&folder, &[primary.clone(), side], &primary).expect("de novo");
        assert_eq!(again, vec![folder.join("lado_2.png").display().to_string()]);
        let _ = std::fs::remove_dir_all(&root);
    }
}
