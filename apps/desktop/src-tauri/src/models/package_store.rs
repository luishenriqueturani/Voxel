//! Árvore `models/hub` e o marcador que o seletor reconhece.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// O que falta gravar quando a última parte chega.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageSpec {
    pub name: String,
    pub opener: String,
    pub directory: bool,
    pub license: String,
    pub parts: Vec<String>,
    pub marker: String,
}

/// Pacote completo já gravado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredPackage {
    pub name: String,
    pub open: String,
    pub bytes: u64,
    pub license: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MarkerBody {
    name: String,
    open: String,
    bytes: u64,
    license: String,
    #[serde(default)]
    parts: Vec<String>,
}

/// Caminho de uma parte dentro de `models/hub/<repo>/`.
///
/// # Errors
///
/// Falha se o caminho sair da pasta de modelos.
pub fn hub_part_path(models: &Path, repo: &str, filename: &str) -> Result<PathBuf, String> {
    safe_relative(models, &format!("hub/{repo}/{filename}"))
}

/// Junta um caminho relativo sem deixar `..` escapar da raiz.
///
/// # Errors
///
/// Falha se o caminho estiver vazio ou tiver um segmento `.` ou `..`.
pub fn safe_relative(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.is_empty() || relative.contains('\0') {
        return Err("nome inválido".to_owned());
    }
    let mut path = root.to_path_buf();
    for part in relative.split(['/', '\\']) {
        if part.is_empty() || part == "." || part == ".." {
            return Err("nome inválido".to_owned());
        }
        path.push(part);
    }
    Ok(path)
}

/// Grava o marcador quando todas as partes já estão no disco.
///
/// # Errors
///
/// Falha se um caminho for inválido ou se a escrita do marcador falhar.
pub fn complete_package(
    models: &Path,
    repo: &str,
    spec: &PackageSpec,
) -> Result<Option<StoredPackage>, String> {
    let repo_dir = safe_relative(models, &format!("hub/{repo}"))?;
    let mut bytes = 0_u64;
    for part in &spec.parts {
        let path = safe_relative(&repo_dir, part)?;
        if !path.is_file() {
            return Ok(None);
        }
        bytes += std::fs::metadata(&path)
            .map_err(|error| error.to_string())?
            .len();
    }
    let open = open_path(repo, spec);
    let target = safe_relative(models, &open)?;
    let marker_path = marker_path(spec, &target)?;
    write_marker(&marker_path, spec, &open, bytes)?;
    Ok(Some(StoredPackage {
        name: spec.name.clone(),
        open,
        bytes,
        license: spec.license.clone(),
    }))
}

/// Lê os marcadores em `models/hub`.
///
/// # Errors
///
/// Falha se a pasta não puder ser lida.
pub fn stored_packages(models: &Path) -> Result<Vec<StoredPackage>, String> {
    let hub = models.join("hub");
    if !hub.is_dir() {
        return Ok(Vec::new());
    }
    let mut found = Vec::new();
    walk(&hub, models, &mut found)?;
    Ok(found)
}

/// Apaga o pacote apontado pelo id, que é o caminho aberto no `-m`.
///
/// # Errors
///
/// Falha se o id sair da pasta ou se a remoção falhar.
pub fn remove_stored_package(models: &Path, id: &str) -> Result<(), String> {
    let path = safe_relative(models, id)?;
    if path.is_file() {
        return remove_file_package(&path);
    }
    if !path.is_dir() {
        return Err("arquivo ausente".to_owned());
    }
    let marker = marker_for(&path, id).ok_or_else(|| "arquivo ausente".to_owned())?;
    let body = read_marker(&marker).ok_or_else(|| "arquivo ausente".to_owned())?;
    if repo_root(id) || markers_in(&path).len() > 1 {
        return remove_listed_parts(&path, &body, &marker);
    }
    std::fs::remove_dir_all(&path).map_err(|error| error.to_string())
}

/// Abre um arquivo solto ou a pasta de um pacote completo.
///
/// # Errors
///
/// Falha se o id sair da pasta ou se o alvo não for um modelo instalado.
pub fn resolve_installed(models: &Path, id: &str) -> Result<PathBuf, String> {
    let path = safe_relative(models, id)?;
    if path.is_dir() && marker_for(&path, id).is_some() {
        return Ok(path);
    }
    if path.is_file() && (flat_name(id) || sidecar(&path).is_file()) {
        return Ok(path);
    }
    Err("modelo ausente".to_owned())
}

fn open_path(repo: &str, spec: &PackageSpec) -> String {
    if spec.opener.is_empty() {
        format!("hub/{repo}")
    } else {
        format!("hub/{repo}/{}", spec.opener)
    }
}

fn marker_path(spec: &PackageSpec, target: &Path) -> Result<PathBuf, String> {
    if spec.directory {
        if !target.is_dir() {
            return Err("pasta ausente".to_owned());
        }
        return Ok(target.join(marker_name(spec)?));
    }
    if !target.is_file() {
        return Err("arquivo ausente".to_owned());
    }
    Ok(sidecar(target))
}

fn marker_name(spec: &PackageSpec) -> Result<String, String> {
    let name = if spec.marker.is_empty() {
        "voxel-package.json".to_owned()
    } else {
        spec.marker.clone()
    };
    if valid_marker_name(&name) {
        Ok(name)
    } else {
        Err("nome inválido".to_owned())
    }
}

fn valid_marker_name(name: &str) -> bool {
    if name == "voxel-package.json" {
        return true;
    }
    let Some(stem) = name.strip_prefix("voxel-package-") else {
        return false;
    };
    let Some(stem) = stem.strip_suffix(".json") else {
        return false;
    };
    !stem.is_empty()
        && stem.chars().all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
        })
}

fn write_marker(path: &Path, spec: &PackageSpec, open: &str, bytes: u64) -> Result<(), String> {
    let body = MarkerBody {
        name: spec.name.clone(),
        open: open.to_owned(),
        bytes,
        license: spec.license.clone(),
        parts: spec.parts.clone(),
    };
    let text = serde_json::to_string(&body).map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(path, text).map_err(|error| error.to_string())
}

fn walk(dir: &Path, models: &Path, out: &mut Vec<StoredPackage>) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            walk(&path, models, out)?;
            continue;
        }
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        if !is_marker_name(name) {
            continue;
        }
        let Some(body) = read_marker(&path) else {
            continue;
        };
        if safe_relative(models, &body.open).is_err()
            || out.iter().any(|item| item.open == body.open)
        {
            continue;
        }
        out.push(StoredPackage {
            name: body.name,
            open: body.open,
            bytes: body.bytes,
            license: body.license,
        });
    }
    Ok(())
}

fn is_marker_name(name: &str) -> bool {
    name == "voxel-package.json" || name.ends_with(".voxel-package.json") || valid_marker_name(name)
}

fn read_marker(path: &Path) -> Option<MarkerBody> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn marker_for(dir: &Path, id: &str) -> Option<PathBuf> {
    markers_in(dir)
        .into_iter()
        .find(|marker| read_marker(marker).is_some_and(|body| body.open == id))
}

fn markers_in(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(is_marker_name)
        })
        .collect()
}

fn remove_listed_parts(repo_dir: &Path, body: &MarkerBody, marker: &Path) -> Result<(), String> {
    for part in &body.parts {
        let file = safe_relative(repo_dir, part)?;
        if file.is_file() {
            std::fs::remove_file(&file).map_err(|error| error.to_string())?;
        }
        let partial = partial_path(&file);
        if partial.is_file() {
            std::fs::remove_file(partial).map_err(|error| error.to_string())?;
        }
    }
    std::fs::remove_file(marker).map_err(|error| error.to_string())
}

fn remove_file_package(path: &Path) -> Result<(), String> {
    std::fs::remove_file(path).map_err(|error| error.to_string())?;
    let marker = sidecar(path);
    if marker.is_file() {
        std::fs::remove_file(marker).map_err(|error| error.to_string())?;
    }
    let partial = partial_path(path);
    if partial.is_file() {
        std::fs::remove_file(partial).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn repo_root(id: &str) -> bool {
    let parts: Vec<&str> = id.split('/').filter(|part| !part.is_empty()).collect();
    parts.len() == 3 && parts[0] == "hub"
}

fn flat_name(id: &str) -> bool {
    !id.contains('/') && !id.contains('\\')
}

fn sidecar(file: &Path) -> PathBuf {
    let mut name = file.as_os_str().to_os_string();
    name.push(".voxel-package.json");
    PathBuf::from(name)
}

fn partial_path(destination: &Path) -> PathBuf {
    let mut name = destination.as_os_str().to_os_string();
    name.push(".partial");
    PathBuf::from(name)
}

#[cfg(test)]
mod tests {
    use super::{PackageSpec, complete_package, resolve_installed, stored_packages};
    use crate::models::{list_models, remove_model};
    use voxel_models::MachineFacts;

    fn spec(parts: Vec<String>) -> PackageSpec {
        PackageSpec {
            name: "gato · merged-model".to_owned(),
            opener: "merged-model".to_owned(),
            directory: true,
            license: "mit".to_owned(),
            parts,
            marker: "voxel-package.json".to_owned(),
        }
    }

    #[test]
    fn o_marcador_so_aparece_com_todas_as_partes() {
        let root = std::env::temp_dir().join(format!("voxel-hub-{}", std::process::id()));
        let models = root.join("models");
        let _ = std::fs::remove_dir_all(&root);
        let repo = "acme/gato";
        let first = "merged-model/vae/diffusion_pytorch_model.safetensors";
        let second = "merged-model/unet/diffusion_pytorch_model.safetensors";
        let pending = spec(vec![first.to_owned(), second.to_owned()]);
        write_part(&models, repo, first);
        assert!(complete_package(&models, repo, &pending).unwrap().is_none());
        assert!(stored_packages(&models).unwrap().is_empty());
        write_part(&models, repo, second);
        let stored = complete_package(&models, repo, &pending).unwrap().unwrap();
        assert_eq!(stored.open, "hub/acme/gato/merged-model");
        assert_eq!(stored_packages(&models).unwrap().len(), 1);
        let facts = MachineFacts {
            vram_bytes: 8 * 1024 * 1024 * 1024,
            ram_bytes: 8 * 1024 * 1024 * 1024,
            vendor: None,
            vulkan: true,
            rocm: false,
            cuda: false,
        };
        let listed = list_models(&models, &facts).unwrap();
        let model = listed
            .iter()
            .find(|item| item.id == "hub/acme/gato/merged-model")
            .unwrap();
        assert_eq!(model.name, "gato · merged-model");
        assert_eq!(model.kind, "image");
        assert!(
            listed
                .iter()
                .all(|item| !item.id.contains("diffusion_pytorch"))
        );
        let opened = resolve_installed(&models, &stored.open).unwrap();
        assert!(opened.is_dir());
        let shard =
            models.join("hub/acme/gato/merged-model/vae/diffusion_pytorch_model.safetensors");
        assert!(
            resolve_installed(
                &models,
                "hub/acme/gato/merged-model/vae/diffusion_pytorch_model.safetensors"
            )
            .is_err()
        );
        assert!(shard.is_file());
        remove_model(&models, &stored.open).unwrap();
        assert!(!opened.exists());
        assert!(resolve_installed(&models, "ausente.safetensors").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    fn write_part(models: &std::path::Path, repo: &str, filename: &str) {
        let path = super::hub_part_path(models, repo, filename).unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "peso").unwrap();
    }
}
