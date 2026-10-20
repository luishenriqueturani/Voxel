use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde_json::Value;

use crate::catalog::ModelError;
use crate::seal::{Compatibility, Reason};

const HEADER_LIMIT: u64 = 32 * 1024 * 1024;

/// Arquitetura lida do cabeçalho e tamanho do arquivo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WeightInfo {
    pub architecture: String,
    pub bytes: u64,
}

/// Estima o selo pelo tamanho do arquivo diante da VRAM.
#[must_use]
pub fn estimate_by_size(bytes: u64, vram_bytes: u64) -> Compatibility {
    if bytes <= vram_bytes && vram_bytes > 0 {
        Compatibility::Compatible
    } else {
        Compatibility::Incompatible(Reason::NeedsVram {
            required_bytes: bytes,
            available_bytes: vram_bytes,
        })
    }
}

/// Lê a arquitetura de um `.safetensors` ou `.gguf` sem carregar os pesos.
///
/// # Errors
///
/// Falha se a extensão, o cabeçalho ou o JSON não forem reconhecidos.
pub fn inspect_file(path: &Path) -> Result<WeightInfo, ModelError> {
    let bytes = std::fs::metadata(path)?.len();
    let architecture = read_architecture(path)?;
    Ok(WeightInfo {
        architecture,
        bytes,
    })
}

fn read_architecture(path: &Path) -> Result<String, ModelError> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| ModelError::Invalid("extensão ausente".to_owned()))?;
    let mut file = File::open(path)?;
    let mut header = Vec::new();
    file.by_ref().take(HEADER_LIMIT).read_to_end(&mut header)?;
    match extension.as_str() {
        "safetensors" => safetensors_architecture(&header),
        "gguf" => gguf_architecture(&header),
        _ => Err(ModelError::Invalid("extensão não suportada".to_owned())),
    }
}

fn safetensors_architecture(bytes: &[u8]) -> Result<String, ModelError> {
    if bytes.len() < 8 {
        return Err(ModelError::Invalid("cabeçalho truncado".to_owned()));
    }
    let length = u64::from_le_bytes(bytes[0..8].try_into().expect("8 bytes"));
    if length == 0 || length > HEADER_LIMIT {
        return Err(ModelError::Invalid("cabeçalho grande".to_owned()));
    }
    let length =
        usize::try_from(length).map_err(|_| ModelError::Invalid("cabeçalho".to_owned()))?;
    let end = 8 + length;
    if bytes.len() < end {
        return Err(ModelError::Invalid("cabeçalho truncado".to_owned()));
    }
    let value: Value = serde_json::from_slice(&bytes[8..end])?;
    Ok(architecture_from_json(&value))
}

fn architecture_from_json(value: &Value) -> String {
    if let Some(meta) = value.get("__metadata__").and_then(Value::as_object) {
        for key in [
            "modelspec.architecture",
            "architecture",
            "general.architecture",
        ] {
            if let Some(text) = meta.get(key).and_then(Value::as_str)
                && !text.is_empty()
            {
                return text.to_owned();
            }
        }
    }
    let names = value
        .as_object()
        .map(|entries| entries.keys().map(String::as_str).collect::<Vec<_>>())
        .unwrap_or_default();
    if names.iter().any(|name| name.contains("diffusion_model")) {
        return "stable-diffusion".to_owned();
    }
    if names.iter().any(|name| name.contains("img_mlp")) {
        return "qwen_image".to_owned();
    }
    "unknown".to_owned()
}

fn gguf_architecture(bytes: &[u8]) -> Result<String, ModelError> {
    if bytes.len() < 4 || &bytes[0..4] != b"GGUF" {
        return Err(ModelError::Invalid("não é gguf".to_owned()));
    }
    let mut cursor = 4;
    let _version = read_u32(bytes, &mut cursor)?;
    let _tensors = read_u64(bytes, &mut cursor)?;
    let pairs = read_u64(bytes, &mut cursor)?;
    if pairs > 10_000 {
        return Err(ModelError::Invalid("metadados grandes".to_owned()));
    }
    for _ in 0..pairs {
        let key = read_string(bytes, &mut cursor)?;
        let kind = read_u32(bytes, &mut cursor)?;
        if key == "general.architecture" && kind == 8 {
            return read_string(bytes, &mut cursor);
        }
        skip_value(bytes, &mut cursor, kind)?;
    }
    Ok("unknown".to_owned())
}

fn read_u32(bytes: &[u8], cursor: &mut usize) -> Result<u32, ModelError> {
    let slice = take(bytes, cursor, 4)?;
    Ok(u32::from_le_bytes(slice.try_into().expect("4 bytes")))
}

fn read_u64(bytes: &[u8], cursor: &mut usize) -> Result<u64, ModelError> {
    let slice = take(bytes, cursor, 8)?;
    Ok(u64::from_le_bytes(slice.try_into().expect("8 bytes")))
}

fn read_string(bytes: &[u8], cursor: &mut usize) -> Result<String, ModelError> {
    let length = read_u64(bytes, cursor)?;
    if length > HEADER_LIMIT {
        return Err(ModelError::Invalid("texto grande".to_owned()));
    }
    let length = usize::try_from(length).map_err(|_| ModelError::Invalid("texto".to_owned()))?;
    let slice = take(bytes, cursor, length)?;
    String::from_utf8(slice.to_vec()).map_err(|_| ModelError::Invalid("texto inválido".to_owned()))
}

fn skip_value(bytes: &[u8], cursor: &mut usize, kind: u32) -> Result<(), ModelError> {
    let size = match kind {
        0 | 1 | 7 => 1,
        2..=3 => 2,
        4..=6 => 4,
        10..=12 => 8,
        8 => {
            let _ = read_string(bytes, cursor)?;
            return Ok(());
        }
        9 => {
            let inner = read_u32(bytes, cursor)?;
            let count = read_u64(bytes, cursor)?;
            if count > 1_000_000 {
                return Err(ModelError::Invalid("lista grande".to_owned()));
            }
            for _ in 0..count {
                skip_value(bytes, cursor, inner)?;
            }
            return Ok(());
        }
        _ => return Err(ModelError::Invalid("tipo desconhecido".to_owned())),
    };
    let _ = take(bytes, cursor, size)?;
    Ok(())
}

fn take<'a>(bytes: &'a [u8], cursor: &mut usize, len: usize) -> Result<&'a [u8], ModelError> {
    let end = cursor
        .checked_add(len)
        .ok_or_else(|| ModelError::Invalid("cabeçalho truncado".to_owned()))?;
    if end > bytes.len() {
        return Err(ModelError::Invalid("cabeçalho truncado".to_owned()));
    }
    let slice = &bytes[*cursor..end];
    *cursor = end;
    Ok(slice)
}

#[cfg(test)]
mod tests {
    use super::{estimate_by_size, gguf_architecture, safetensors_architecture};
    use crate::seal::{Compatibility, Reason};

    #[test]
    fn o_tamanho_maior_que_a_vram_e_incompativel() {
        assert_eq!(estimate_by_size(100, 1000), Compatibility::Compatible);
        assert_eq!(
            estimate_by_size(5_000, 1_000),
            Compatibility::Incompatible(Reason::NeedsVram {
                required_bytes: 5_000,
                available_bytes: 1_000,
            })
        );
        assert!(matches!(
            estimate_by_size(10, 0),
            Compatibility::Incompatible(_)
        ));
    }

    #[test]
    fn le_a_arquitetura_do_safetensors_e_do_gguf() {
        let header = br#"{"__metadata__":{"modelspec.architecture":"stable-diffusion-v1"},"weight":{"dtype":"F32","shape":[1],"data_offsets":[0,4]}}"#;
        let mut bytes = (header.len() as u64).to_le_bytes().to_vec();
        bytes.extend_from_slice(header);
        assert_eq!(
            safetensors_architecture(&bytes).unwrap(),
            "stable-diffusion-v1"
        );

        let named =
            br#"{"model.diffusion_model.weight":{"dtype":"F32","shape":[1],"data_offsets":[0,4]}}"#;
        let mut named_bytes = (named.len() as u64).to_le_bytes().to_vec();
        named_bytes.extend_from_slice(named);
        assert_eq!(
            safetensors_architecture(&named_bytes).unwrap(),
            "stable-diffusion"
        );

        let qwen = br#"{"transformer_blocks.0.img_mlp.proj.weight":{"dtype":"F32","shape":[1],"data_offsets":[0,4]}}"#;
        let mut qwen_bytes = (qwen.len() as u64).to_le_bytes().to_vec();
        qwen_bytes.extend_from_slice(qwen);
        assert_eq!(safetensors_architecture(&qwen_bytes).unwrap(), "qwen_image");

        let mesh =
            br#"{"image_tokenizer.weight":{"dtype":"F32","shape":[1],"data_offsets":[0,4]}}"#;
        let mut mesh_bytes = (mesh.len() as u64).to_le_bytes().to_vec();
        mesh_bytes.extend_from_slice(mesh);
        assert_eq!(safetensors_architecture(&mesh_bytes).unwrap(), "unknown");

        let mut gguf = b"GGUF".to_vec();
        gguf.extend_from_slice(&3_u32.to_le_bytes());
        gguf.extend_from_slice(&0_u64.to_le_bytes());
        gguf.extend_from_slice(&2_u64.to_le_bytes());
        push_string(&mut gguf, "general.alignment");
        gguf.extend_from_slice(&4_u32.to_le_bytes());
        gguf.extend_from_slice(&32_u32.to_le_bytes());
        push_string(&mut gguf, "general.architecture");
        gguf.extend_from_slice(&8_u32.to_le_bytes());
        push_string(&mut gguf, "stable-diffusion");
        assert_eq!(gguf_architecture(&gguf).unwrap(), "stable-diffusion");
    }

    fn push_string(bytes: &mut Vec<u8>, text: &str) {
        bytes.extend_from_slice(&(text.len() as u64).to_le_bytes());
        bytes.extend_from_slice(text.as_bytes());
    }
}
