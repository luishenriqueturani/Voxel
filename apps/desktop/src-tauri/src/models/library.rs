use std::path::{Path, PathBuf};

use serde::Serialize;
use voxel_hardware::{Vendor, detect};
use voxel_models::{
    Catalog, Compatibility, GpuVendor, MachineFacts, Reason, compatibility, estimate_by_size,
    inspect_file,
};

/// Modelo do catálogo ou importado, com o selo desta máquina.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedModel {
    pub id: String,
    pub name: String,
    pub license: String,
    pub bytes: u64,
    pub filename: String,
    pub installed: bool,
    pub catalog: bool,
    pub compatibility: &'static str,
    pub reason: Option<ListedReason>,
    pub architecture: Option<String>,
    pub kind: String,
}

/// Motivo traduzível do selo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListedReason {
    pub code: &'static str,
    pub required_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub proven_bytes: Option<u64>,
}

/// Lê a máquina atual para o selo.
#[must_use]
pub fn current_facts() -> MachineFacts {
    let report = detect();
    let gpu = report.gpus.iter().max_by_key(|gpu| gpu.vram_bytes);
    MachineFacts {
        vram_bytes: gpu.map_or(0, |gpu| gpu.vram_bytes),
        ram_bytes: report.memory.total_bytes,
        vendor: gpu.and_then(|gpu| match gpu.vendor {
            Vendor::Amd => Some(GpuVendor::Amd),
            Vendor::Nvidia => Some(GpuVendor::Nvidia),
            Vendor::Intel => Some(GpuVendor::Intel),
            Vendor::Other(_) => None,
        }),
        vulkan: gpu.is_some(),
        rocm: report.rocm,
        cuda: report.cuda,
    }
}

/// Lista o catálogo e os arquivos da pasta de modelos.
///
/// # Errors
///
/// Falha se o catálogo ou a pasta não puderem ser lidos.
pub fn list_models(dir: &Path, facts: &MachineFacts) -> Result<Vec<ListedModel>, String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let catalog = Catalog::bundled().map_err(|error| error.to_string())?;
    let mut listed = Vec::new();
    for model in &catalog.models {
        let filename = model.filename.clone();
        let installed = dir.join(&filename).is_file();
        listed.push(describe(Entry {
            id: &model.id,
            name: &model.name,
            license: &model.license,
            bytes: model.bytes,
            filename: &filename,
            installed,
            catalog: true,
            seal: compatibility(model, facts),
            architecture: None,
            kind: kind_name(model.kind),
        }));
    }
    let mut extras = std::fs::read_dir(dir)
        .map_err(|error| error.to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            is_weight(path)
                && !catalog
                    .models
                    .iter()
                    .any(|model| path.ends_with(&model.filename))
        })
        .collect::<Vec<_>>();
    extras.sort();
    for path in extras {
        let Some(filename) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        let info = inspect_file(&path).ok();
        let bytes = info
            .as_ref()
            .map(|info| info.bytes)
            .or_else(|| path.metadata().ok().map(|meta| meta.len()))
            .unwrap_or(0);
        let architecture = info.map(|info| info.architecture);
        listed.push(describe(Entry {
            id: filename,
            name: filename,
            license: "",
            bytes,
            filename,
            installed: true,
            catalog: false,
            seal: estimate_by_size(bytes, facts.vram_bytes),
            kind: import_kind(architecture.as_deref()),
            architecture,
        }));
    }
    Ok(listed)
}

/// Lê a arquitetura do arquivo já gravado e estima o selo pelo tamanho.
///
/// # Errors
///
/// Falha se o nome sair da pasta ou se o cabeçalho não for reconhecido.
pub fn describe_downloaded(
    dir: &Path,
    filename: &str,
    license: &str,
    facts: &MachineFacts,
) -> Result<ListedModel, String> {
    let path = file_in_dir(dir, filename)?;
    if !path.is_file() {
        return Err("arquivo ausente".to_owned());
    }
    let info = inspect_file(&path).map_err(|error| error.to_string())?;
    Ok(describe(Entry {
        id: filename,
        name: filename,
        license,
        bytes: info.bytes,
        filename,
        installed: true,
        catalog: false,
        seal: estimate_by_size(info.bytes, facts.vram_bytes),
        kind: import_kind(Some(info.architecture.as_str())),
        architecture: Some(info.architecture),
    }))
}

/// Copia um `.safetensors` ou `.gguf` para a pasta de modelos.
///
/// # Errors
///
/// Falha se a extensão não servir, o arquivo já existir ou a cópia falhar.
pub fn import_model(
    dir: &Path,
    source: &Path,
    facts: &MachineFacts,
) -> Result<ListedModel, String> {
    if !is_weight(source) {
        return Err("extensão não suportada".to_owned());
    }
    let filename = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("nome inválido")?
        .to_owned();
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    let destination = dir.join(&filename);
    if destination.is_file() {
        return Err("arquivo já existe".to_owned());
    }
    let info = inspect_file(source).map_err(|error| error.to_string())?;
    std::fs::copy(source, &destination).map_err(|error| error.to_string())?;
    Ok(describe(Entry {
        id: &filename,
        name: &filename,
        license: "",
        bytes: info.bytes,
        filename: &filename,
        installed: true,
        catalog: false,
        seal: estimate_by_size(info.bytes, facts.vram_bytes),
        kind: import_kind(Some(info.architecture.as_str())),
        architecture: Some(info.architecture),
    }))
}

/// Apaga um arquivo da pasta de modelos.
///
/// # Errors
///
/// Falha se o nome sair da pasta ou se a remoção falhar.
pub fn remove_model(dir: &Path, filename: &str) -> Result<(), String> {
    let path = file_in_dir(dir, filename)?;
    if path.is_file() {
        std::fs::remove_file(&path).map_err(|error| error.to_string())?;
    } else {
        return Err("arquivo ausente".to_owned());
    }
    let partial = partial_path(&path);
    if partial.is_file() {
        std::fs::remove_file(partial).map_err(|error| error.to_string())?;
    }
    Ok(())
}

/// Cria a pasta de modelos e devolve o caminho.
///
/// # Errors
///
/// Falha se a pasta não puder ser criada.
pub fn ensure_models_dir(dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    Ok(dir.to_path_buf())
}

struct Entry<'a> {
    id: &'a str,
    name: &'a str,
    license: &'a str,
    bytes: u64,
    filename: &'a str,
    installed: bool,
    catalog: bool,
    seal: Compatibility,
    architecture: Option<String>,
    kind: &'static str,
}

fn describe(entry: Entry<'_>) -> ListedModel {
    let (compatibility, reason) = match entry.seal {
        Compatibility::Compatible => ("compatible", None),
        Compatibility::Caveat(reason) => ("caveat", Some(view_reason(reason))),
        Compatibility::Incompatible(reason) => ("incompatible", Some(view_reason(reason))),
    };
    ListedModel {
        id: entry.id.to_owned(),
        name: entry.name.to_owned(),
        license: entry.license.to_owned(),
        bytes: entry.bytes,
        filename: entry.filename.to_owned(),
        installed: entry.installed,
        catalog: entry.catalog,
        compatibility,
        reason,
        architecture: entry.architecture,
        kind: entry.kind.to_owned(),
    }
}

fn kind_name(kind: voxel_models::Kind) -> &'static str {
    match kind {
        voxel_models::Kind::Image => "image",
        voxel_models::Kind::Mesh => "mesh",
        voxel_models::Kind::Background => "background",
    }
}

fn import_kind(architecture: Option<&str>) -> &'static str {
    let name = architecture.unwrap_or("").to_ascii_lowercase();
    if name.contains("diffusion")
        || name.contains("stable")
        || name.contains("flux")
        || name.contains("image")
    {
        "image"
    } else {
        "unknown"
    }
}

fn view_reason(reason: Reason) -> ListedReason {
    let (code, required_bytes, available_bytes, proven_bytes) = match reason {
        Reason::RequiresCuda => ("requires_cuda", None, None, None),
        Reason::NeedsVram {
            required_bytes,
            available_bytes,
        } => (
            "needs_vram",
            Some(required_bytes),
            Some(available_bytes),
            None,
        ),
        Reason::NeedsRam {
            required_bytes,
            available_bytes,
        } => (
            "needs_ram",
            Some(required_bytes),
            Some(available_bytes),
            None,
        ),
        Reason::OffloadRam => ("offload_ram", None, None, None),
        Reason::CpuSlow => ("cpu_slow", None, None, None),
        Reason::ExperimentalAttention => ("experimental_attention", None, None, None),
        Reason::UnprovenVram {
            proven_bytes,
            available_bytes,
        } => (
            "unproven_vram",
            None,
            Some(available_bytes),
            Some(proven_bytes),
        ),
    };
    ListedReason {
        code,
        required_bytes,
        available_bytes,
        proven_bytes,
    }
}

fn is_weight(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("safetensors" | "gguf")
    )
}

fn file_in_dir(dir: &Path, filename: &str) -> Result<PathBuf, String> {
    if filename.is_empty()
        || filename.contains('/')
        || filename.contains('\\')
        || filename.contains('\0')
        || filename == "."
        || filename == ".."
    {
        return Err("nome inválido".to_owned());
    }
    Ok(dir.join(filename))
}

fn partial_path(destination: &Path) -> PathBuf {
    let mut name = destination.as_os_str().to_os_string();
    name.push(".partial");
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::{describe_downloaded, import_kind, import_model, list_models, remove_model};
    use voxel_models::MachineFacts;

    fn facts(vram: u64) -> MachineFacts {
        MachineFacts {
            vram_bytes: vram,
            ram_bytes: 32 * 1024 * 1024 * 1024,
            vendor: None,
            vulkan: vram > 0,
            rocm: false,
            cuda: false,
        }
    }

    fn sample(dir: &std::path::Path) -> std::path::PathBuf {
        let header = br#"{"__metadata__":{"modelspec.architecture":"stable-diffusion-v1"},"weight":{"dtype":"F32","shape":[1],"data_offsets":[0,4]}}"#;
        let mut bytes = (header.len() as u64).to_le_bytes().to_vec();
        bytes.extend_from_slice(header);
        bytes.extend_from_slice(&[0, 0, 0, 0]);
        let path = dir.join("local.safetensors");
        std::fs::write(&path, bytes).unwrap();
        path
    }

    #[test]
    fn importa_mostra_a_arquitetura_e_remove() {
        let root = std::env::temp_dir().join(format!("voxel-models-{}", std::process::id()));
        let source_dir = root.join("origem");
        let models = root.join("models");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&source_dir).unwrap();
        let source = sample(&source_dir);
        let imported = import_model(&models, &source, &facts(10_000_000)).unwrap();
        assert_eq!(
            imported.architecture.as_deref(),
            Some("stable-diffusion-v1")
        );
        assert_eq!(imported.compatibility, "compatible");
        let described =
            describe_downloaded(&models, "local.safetensors", "mit", &facts(10_000_000)).unwrap();
        assert_eq!(described.license, "mit");
        assert!(!described.catalog);
        assert_eq!(
            described.architecture.as_deref(),
            Some("stable-diffusion-v1")
        );
        assert!(import_model(&models, &source, &facts(10_000_000)).is_err());
        let small = list_models(&models, &facts(1)).unwrap();
        let local = small
            .iter()
            .find(|model| model.id == "local.safetensors")
            .unwrap();
        assert_eq!(local.compatibility, "incompatible");
        remove_model(&models, "local.safetensors").unwrap();
        assert!(remove_model(&models, "../local.safetensors").is_err());
        let after = list_models(&models, &facts(10_000_000)).unwrap();
        assert!(after.iter().all(|model| model.id != "local.safetensors"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn qwen_image_gera_imagem_e_um_modelo_de_texto_nao() {
        assert_eq!(import_kind(Some("qwen_image21")), "image");
        assert_eq!(import_kind(Some("stable-diffusion-xl")), "image");
        assert_eq!(import_kind(Some("qwen2")), "unknown");
        assert_eq!(import_kind(None), "unknown");
    }
}
