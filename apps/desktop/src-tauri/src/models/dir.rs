//! Pasta de modelos: padrão XDG ou o caminho escolhido, com opção de mover os arquivos.

use std::fs;
use std::path::{Path, PathBuf};

use crate::db::Database;
use crate::paths::AppPaths;

/// Pasta efetiva dos modelos.
#[must_use]
pub fn folder(paths: &AppPaths, database: &Database) -> PathBuf {
    match database.models_dir() {
        Ok(Some(path)) if Path::new(&path).is_absolute() => PathBuf::from(path),
        _ => default_folder(paths),
    }
}

/// Pasta efetiva a partir do estado do app, ou o padrão se o banco não estiver pronto.
#[must_use]
pub fn folder_from(app: &tauri::AppHandle, paths: &AppPaths) -> PathBuf {
    use tauri::Manager;
    app.try_state::<Database>().map_or_else(
        || default_folder(paths),
        |database| folder(paths, &database),
    )
}

/// `~/.local/share/voxel/models`, salvo se `XDG_DATA_HOME` apontar para outro lugar.
#[must_use]
pub fn default_folder(paths: &AppPaths) -> PathBuf {
    paths.data.join("models")
}

/// Grava a pasta e, se pedido, leva os arquivos já baixados.
///
/// # Errors
///
/// Falha se o caminho for relativo, se o destino estiver dentro da origem, ou se a cópia falhar.
pub fn set_folder(
    paths: &AppPaths,
    database: &Database,
    destination: &Path,
    move_files: bool,
) -> Result<PathBuf, String> {
    if !destination.is_absolute() {
        return Err("pasta relativa".to_owned());
    }
    let source = folder(paths, database);
    if same_dir(&source, destination) {
        return super::ensure_models_dir(destination);
    }
    if move_files && destination.starts_with(&source) {
        return Err("destino dentro da pasta atual".to_owned());
    }
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    if move_files {
        relocate(&source, destination)?;
    } else {
        super::ensure_models_dir(destination)?;
    }
    let stored = if destination == default_folder(paths).as_path() {
        None
    } else {
        Some(destination.display().to_string())
    };
    database
        .set_models_dir(stored.as_deref())
        .map_err(|error| error.to_string())?;
    super::ensure_models_dir(destination)
}

fn relocate(source: &Path, destination: &Path) -> Result<(), String> {
    if !source.exists() {
        super::ensure_models_dir(destination)?;
        return Ok(());
    }
    if destination.exists() {
        copy_tree(source, destination)?;
        return fs::remove_dir_all(source).map_err(|error| error.to_string());
    }
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(error) if crosses_devices(&error) => {
            copy_tree(source, destination)?;
            fs::remove_dir_all(source).map_err(|error| error.to_string())
        }
        Err(error) => Err(error.to_string()),
    }
}

fn crosses_devices(error: &std::io::Error) -> bool {
    error.raw_os_error() == Some(18)
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    fs::create_dir_all(to).map_err(|error| error.to_string())?;
    for entry in fs::read_dir(from).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let dest = to.join(entry.file_name());
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        if kind.is_dir() {
            copy_tree(&entry.path(), &dest)?;
        } else {
            fs::copy(entry.path(), dest).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn same_dir(left: &Path, right: &Path) -> bool {
    let ok_left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let ok_right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    ok_left == ok_right
}

#[cfg(test)]
mod tests {
    use super::{default_folder, folder, set_folder};
    use crate::db::Database;
    use crate::paths::AppPaths;
    use std::path::Path;

    fn paths(root: &std::path::Path) -> AppPaths {
        AppPaths {
            config: root.join("config"),
            data: root.join("data"),
            state: root.join("state"),
            cache: root.join("cache"),
        }
    }

    fn database() -> Database {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        database
    }

    #[test]
    fn sem_ajuste_usa_a_pasta_padrao() {
        let root = std::env::temp_dir().join(format!("voxel-mdir-{}", std::process::id()));
        let paths = paths(&root);
        let database = database();
        assert_eq!(folder(&paths, &database), default_folder(&paths));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_troca_move_os_arquivos_e_passa_a_ler_o_destino() {
        let root = std::env::temp_dir().join(format!("voxel-mmove-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let paths = paths(&root);
        let source = default_folder(&paths);
        std::fs::create_dir_all(source.join("hub/acme")).unwrap();
        std::fs::write(source.join("peso.safetensors"), b"x").unwrap();
        std::fs::write(source.join("hub/acme/a.gguf"), b"y").unwrap();
        let database = database();
        let dest = root.join("outros/modelos");
        let saved = set_folder(&paths, &database, &dest, true).unwrap();
        assert_eq!(saved, dest);
        assert!(dest.join("peso.safetensors").is_file());
        assert!(dest.join("hub/acme/a.gguf").is_file());
        assert!(!source.join("peso.safetensors").exists());
        assert_eq!(folder(&paths, &database), dest);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn recusa_caminho_relativo() {
        let root = std::env::temp_dir().join(format!("voxel-mrel-{}", std::process::id()));
        let paths = paths(&root);
        let database = database();
        assert!(set_folder(&paths, &database, Path::new("modelos"), false).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
