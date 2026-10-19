//! Pasta de saída, `metadados.json` e o bloco de texto embutido no PNG.
//!
//! A árvore segue o planejamento: `Imagens/Voxel/AAAA-MM-DD/001_slug/imagem_01.png`.
//! O JSON da imagem entra no chunk `iTXt` antes do `IEND`, com a palavra `voxel`.

use std::path::{Path, PathBuf};

use serde::Serialize;

const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";
const TEXT_KEYWORD: &str = "voxel";

/// Metadados de uma imagem gerada. O mesmo conteúdo vai para o JSON e para o PNG.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageMetadata {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub model_id: String,
    pub backend: String,
    pub seed: i64,
    pub steps: i64,
    pub width: i64,
    pub height: i64,
    pub cfg: f64,
    pub elapsed_ms: i64,
    pub created_at: String,
    pub file: String,
}

/// `~/Imagens/Voxel`, ou o que `XDG_PICTURES_DIR` apontar, mais `Voxel`.
#[must_use]
pub fn pictures_root(home: &Path, user_dirs: &str) -> PathBuf {
    for line in user_dirs.lines() {
        let Some(value) = line.trim().strip_prefix("XDG_PICTURES_DIR=") else {
            continue;
        };
        let value = value.trim().trim_matches('"');
        let expanded = value.replace("$HOME", &home.display().to_string());
        let path = PathBuf::from(expanded);
        if path.is_absolute() {
            return path.join("Voxel");
        }
    }
    home.join("Imagens").join("Voxel")
}

/// Diretório do dia a partir do relógio do sistema, em UTC.
#[must_use]
pub fn today_dir(root: &Path) -> PathBuf {
    root.join(utc_date(unix_now()))
}

/// Próxima pasta `001_slug` dentro do dia. Não cria o diretório.
#[must_use]
pub fn item_directory(day_dir: &Path, prompt: &str) -> PathBuf {
    let next = std::fs::read_dir(day_dir)
        .ok()
        .map_or(0, |entries| {
            entries
                .filter_map(Result::ok)
                .filter(|entry| entry.path().is_dir())
                .count()
        })
        .saturating_add(1);
    day_dir.join(format!("{next:03}_{}", slug(prompt)))
}

/// Data UTC `AAAA-MM-DD` a partir de segundos Unix.
#[must_use]
pub fn utc_date(unix_seconds: i64) -> String {
    let z = unix_seconds.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = u64::try_from(z - era * 146_097).unwrap_or(0);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year_base = i64::try_from(yoe).unwrap_or(0) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year_base + 1 } else { year_base };
    format!("{year:04}-{month:02}-{day:02}")
}

/// Instante UTC `AAAA-MM-DDTHH:MM:SSZ`.
#[must_use]
pub fn utc_timestamp(unix_seconds: i64) -> String {
    let day = utc_date(unix_seconds);
    let seconds = unix_seconds.rem_euclid(86_400);
    let hour = seconds / 3_600;
    let minute = (seconds % 3_600) / 60;
    let second = seconds % 60;
    format!("{day}T{hour:02}:{minute:02}:{second:02}Z")
}

/// Slug curto do prompt, só com letras e números ASCII.
#[must_use]
pub fn slug(prompt: &str) -> String {
    let mut out = String::new();
    for character in prompt.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
        } else if character.is_whitespace() && !out.is_empty() && !out.ends_with('_') {
            out.push('_');
        }
        if out.len() >= 24 {
            break;
        }
    }
    let trimmed = out.trim_matches('_');
    if trimmed.is_empty() {
        "imagem".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// Grava a lista de imagens em `metadados.json`.
///
/// # Errors
///
/// Falha se o diretório não puder ser criado ou o JSON não puder ser escrito.
pub fn write_metadata(folder: &Path, images: &[ImageMetadata]) -> Result<(), String> {
    std::fs::create_dir_all(folder).map_err(|error| error.to_string())?;
    let json = serde_json::to_string_pretty(images).map_err(|error| error.to_string())?;
    std::fs::write(folder.join("metadados.json"), json).map_err(|error| error.to_string())
}

/// Embute o JSON no PNG que o worker acabou de gravar.
///
/// # Errors
///
/// Falha se o arquivo não for um PNG ou não puder ser reescrito.
pub fn embed_metadata(path: &Path, metadata: &ImageMetadata) -> Result<(), String> {
    let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
    let json = serde_json::to_string(metadata).map_err(|error| error.to_string())?;
    let embedded = embed_text(&bytes, &json)?;
    std::fs::write(path, embedded).map_err(|error| error.to_string())
}

/// Insere um chunk `iTXt` com a palavra `voxel` antes do `IEND`.
///
/// # Errors
///
/// Falha se os bytes não forem um PNG com `IEND`.
pub fn embed_text(png: &[u8], text: &str) -> Result<Vec<u8>, String> {
    let insert_at = iend_offset(png)?;
    let chunk = itxt_chunk(TEXT_KEYWORD, text);
    let mut out = Vec::with_capacity(png.len() + chunk.len());
    out.extend_from_slice(&png[..insert_at]);
    out.extend_from_slice(&chunk);
    out.extend_from_slice(&png[insert_at..]);
    Ok(out)
}

/// Lê o texto `voxel` do primeiro chunk `iTXt` com essa palavra.
#[cfg(test)]
#[must_use]
pub(crate) fn read_text(png: &[u8]) -> Option<String> {
    let mut cursor = PNG_SIGNATURE.len();
    while cursor + 12 <= png.len() {
        let length = usize::try_from(u32::from_be_bytes(png[cursor..cursor + 4].try_into().ok()?))
            .unwrap_or(usize::MAX);
        let kind = png.get(cursor + 4..cursor + 8)?;
        let data_at = cursor + 8;
        let next = data_at.checked_add(length)?.checked_add(4)?;
        if next > png.len() {
            return None;
        }
        if kind == b"iTXt"
            && let Some(text) = itxt_text(png.get(data_at..data_at + length)?, TEXT_KEYWORD)
        {
            return Some(text);
        }
        cursor = next;
    }
    None
}

/// Copia um PNG gerado para o destino escolhido no diálogo de salvar.
///
/// # Errors
///
/// Falha se a origem não for uma imagem desta pasta de saída ou a cópia falhar.
pub fn copy_generation(source: &Path, destination: &Path) -> Result<(), String> {
    let name = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let sibling = source
        .parent()
        .is_some_and(|dir| dir.join("metadados.json").is_file());
    let png = Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"));
    if !name.starts_with("imagem_") || !png || !sibling {
        return Err("origem inválida".to_owned());
    }
    if destination.extension().and_then(|ext| ext.to_str()) != Some("png") {
        return Err("destino inválido".to_owned());
    }
    if let Some(parent) = destination
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::copy(source, destination).map_err(|error| error.to_string())?;
    Ok(())
}

/// Segundos Unix de agora. Zero se o relógio estiver antes da época.
#[must_use]
pub fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| i64::try_from(duration.as_secs()).unwrap_or(0))
}

fn iend_offset(png: &[u8]) -> Result<usize, String> {
    if !png.starts_with(PNG_SIGNATURE) {
        return Err("png inválido".to_owned());
    }
    let mut cursor = PNG_SIGNATURE.len();
    while cursor + 8 <= png.len() {
        let length = usize::try_from(u32::from_be_bytes(
            png[cursor..cursor + 4]
                .try_into()
                .map_err(|_| "png inválido".to_owned())?,
        ))
        .unwrap_or(usize::MAX);
        let kind = &png[cursor + 4..cursor + 8];
        if kind == b"IEND" {
            return Ok(cursor);
        }
        cursor = cursor
            .checked_add(12)
            .and_then(|next| next.checked_add(length))
            .ok_or_else(|| "png inválido".to_owned())?;
        if cursor > png.len() {
            return Err("png inválido".to_owned());
        }
    }
    Err("sem IEND".to_owned())
}

fn itxt_chunk(keyword: &str, text: &str) -> Vec<u8> {
    let mut data = Vec::new();
    data.extend_from_slice(keyword.as_bytes());
    data.push(0);
    data.push(0);
    data.push(0);
    data.push(0);
    data.push(0);
    data.extend_from_slice(text.as_bytes());
    chunk(*b"iTXt", &data)
}

#[cfg(test)]
fn itxt_text(data: &[u8], keyword: &str) -> Option<String> {
    let split = data.iter().position(|byte| *byte == 0)?;
    let (found, rest) = data.split_at(split);
    let rest = rest.get(1..)?;
    if found != keyword.as_bytes() || rest.len() < 4 {
        return None;
    }
    let text = rest.get(4..)?;
    String::from_utf8(text.to_vec()).ok()
}

fn chunk(kind: [u8; 4], data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(12 + data.len());
    let length = u32::try_from(data.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(&kind);
    out.extend_from_slice(data);
    let mut signed = kind.to_vec();
    signed.extend_from_slice(data);
    out.extend_from_slice(&crc32(&signed).to_be_bytes());
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffff_u32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = if crc & 1 == 1 { 0xedb8_8320 } else { 0 };
            crc = (crc >> 1) ^ mask;
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{
        ImageMetadata, copy_generation, crc32, embed_text, item_directory, pictures_root,
        read_text, slug, utc_date, utc_timestamp, write_metadata,
    };

    fn tiny_png() -> Vec<u8> {
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend(super::chunk(*b"IEND", &[]));
        png
    }

    fn sample() -> ImageMetadata {
        ImageMetadata {
            prompt: "um gato".to_owned(),
            negative_prompt: None,
            model_id: "sd15".to_owned(),
            backend: "vulkan".to_owned(),
            seed: 7,
            steps: 20,
            width: 512,
            height: 512,
            cfg: 7.0,
            elapsed_ms: 10,
            created_at: "2026-10-19T00:00:00Z".to_owned(),
            file: "imagem_01.png".to_owned(),
        }
    }

    #[test]
    fn a_data_utc_e_o_slug_saem_do_relogio_e_do_prompt() {
        assert_eq!(utc_date(0), "1970-01-01");
        assert_eq!(utc_date(1_704_067_200), "2024-01-01");
        assert_eq!(utc_timestamp(86_400 + 3_660), "1970-01-02T01:01:00Z");
        assert_eq!(slug("Um gato laranja!"), "um_gato_laranja");
        assert_eq!(slug("!!!"), "imagem");
    }

    #[test]
    fn a_pasta_de_imagens_respeita_o_user_dirs() {
        let home = Path::new("/home/luis");
        let text = "XDG_PICTURES_DIR=\"$HOME/Imagens\"\n";
        assert_eq!(
            pictures_root(home, text),
            Path::new("/home/luis/Imagens/Voxel")
        );
        assert_eq!(
            pictures_root(home, "XDG_PICTURES_DIR=\"relativo\"\n"),
            Path::new("/home/luis/Imagens/Voxel")
        );
    }

    #[test]
    fn o_png_guarda_o_json_e_a_pasta_numera_o_item() {
        assert_eq!(crc32(b"IEND"), 0xAE42_6082);
        let png = embed_text(&tiny_png(), "café").unwrap();
        assert_eq!(read_text(&png).as_deref(), Some("café"));

        let root = std::env::temp_dir().join(format!("voxel-out-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let day = root.join("2026-10-19");
        std::fs::create_dir_all(&day).unwrap();
        let first = item_directory(&day, "um gato");
        std::fs::create_dir_all(&first).unwrap();
        let second = item_directory(&day, "um gato");
        assert!(first.ends_with("001_um_gato"));
        assert!(second.ends_with("002_um_gato"));

        write_metadata(&first, &[sample()]).unwrap();
        let png_path = first.join("imagem_01.png");
        std::fs::write(&png_path, tiny_png()).unwrap();
        let copy = root.join("copia.png");
        copy_generation(&png_path, &copy).unwrap();
        assert!(copy.is_file());
        assert!(copy_generation(&png_path, &root.join("copia.jpg")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
