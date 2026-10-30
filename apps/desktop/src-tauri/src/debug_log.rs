//! Arquivo gravado quando o modal de erro abre. O texto chega pronto da interface.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Grava o relatório em `directory/erro_<millis>.log`.
///
/// # Errors
///
/// Falha se o diretório ou o arquivo não puder ser criado.
pub fn write_debug(directory: &Path, body: &str) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(directory)?;
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    let path = directory.join(format!("erro_{millis}.log"));
    std::fs::write(&path, body)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grava_o_relatorio_no_diretorio() {
        let directory = std::env::temp_dir().join(format!("voxel-debug-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        let path = write_debug(&directory, "Quando: hoje\nModelo: sd15\n").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("Modelo: sd15"));
        assert!(path.starts_with(&directory));
        let _ = std::fs::remove_dir_all(&directory);
    }
}
