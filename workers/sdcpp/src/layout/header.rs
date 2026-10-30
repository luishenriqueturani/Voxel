//! Lê o cabeçalho do GGUF ou do safetensors para achar um transformer solto ou um `LoRA`.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// Arquivo só do transformer (Flux ou Qwen Image), sem VAE nem codificador de texto.
pub fn bare_diffusion(path: &Path) -> bool {
    if !path.is_file() || is_lora(path) {
        return false;
    }
    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) if ext.eq_ignore_ascii_case("gguf") => gguf_is_bare_diffusion(path),
        Some(ext) if ext.eq_ignore_ascii_case("safetensors") => safetensors_is_bare_flux(path),
        _ => false,
    }
}

/// GGUF do Qwen Image / Qwen Image 2.1 só com o transformer.
pub fn is_qwen_image(path: &Path) -> bool {
    gguf_architecture(path).is_some_and(|arch| arch.starts_with("qwen_image"))
}

/// Adaptador `LoRA`: não gera sozinho.
pub fn is_lora(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    let named = path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_ascii_lowercase().contains("lora"));
    if path
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("safetensors"))
    {
        return named || safetensors_is_lora(path);
    }
    named
}

fn safetensors_is_bare_flux(path: &Path) -> bool {
    let Some(serde_json::Value::Object(map)) = safetensors_header(path) else {
        return false;
    };
    bare_names(map.keys().map(String::as_str))
}

fn bare_names<'a>(names: impl IntoIterator<Item = &'a str>) -> bool {
    let mut bare = false;
    for name in names {
        if name.starts_with("model.diffusion_model.") {
            return false;
        }
        bare |= name.starts_with("double_blocks.") || name.starts_with("single_blocks.");
    }
    bare
}

fn safetensors_is_lora(path: &Path) -> bool {
    let Some(serde_json::Value::Object(map)) = safetensors_header(path) else {
        return false;
    };
    map.keys()
        .any(|name| name.contains("lora_A") || name.contains("lora_B") || name.contains(".lora_"))
}

fn safetensors_header(path: &Path) -> Option<serde_json::Value> {
    let file = File::open(path).ok()?;
    let mut input = BufReader::new(file);
    let len = read_u64(&mut input)?;
    if len > 100 * 1_048_576 {
        return None;
    }
    let len = usize::try_from(len).ok()?;
    let mut buf = vec![0; len];
    input.read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

fn gguf_architecture(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut input = BufReader::new(file);
    let mut magic = [0; 4];
    if input.read_exact(&mut magic).is_err() || &magic != b"GGUF" {
        return None;
    }
    let version = read_u32(&mut input)?;
    if version < 2 {
        return None;
    }
    let (tensors, pairs) = (read_u64(&mut input)?, read_u64(&mut input)?);
    let mut architecture = String::new();
    for _ in 0..pairs {
        let key = read_string(&mut input)?;
        let kind = read_u32(&mut input)?;
        if key == "general.architecture" && kind == STRING {
            architecture = read_string(&mut input).unwrap_or_default();
        } else if !skip_value(&mut input, kind) {
            return None;
        }
    }
    let _ = tensors;
    Some(architecture)
}

fn gguf_is_bare_diffusion(path: &Path) -> bool {
    let Some(architecture) = gguf_architecture(path) else {
        return false;
    };
    if architecture.starts_with("qwen_image") {
        return true;
    }
    if architecture != "flux" {
        return false;
    }
    let file = File::open(path).ok();
    let Some(file) = file else {
        return false;
    };
    let mut input = BufReader::new(file);
    let mut magic = [0; 4];
    if input.read_exact(&mut magic).is_err() || &magic != b"GGUF" {
        return false;
    }
    let Some(version) = read_u32(&mut input) else {
        return false;
    };
    if version < 2 {
        return false;
    }
    let (Some(tensors), Some(pairs)) = (read_u64(&mut input), read_u64(&mut input)) else {
        return false;
    };
    for _ in 0..pairs {
        if read_string(&mut input).is_none() {
            return false;
        }
        let Some(kind) = read_u32(&mut input) else {
            return false;
        };
        if !skip_value(&mut input, kind) {
            return false;
        }
    }
    let mut bare = false;
    let mut prefixed = false;
    for _ in 0..tensors {
        let Some(name) = read_string(&mut input) else {
            return false;
        };
        if !skip_tensor_info(&mut input) {
            return false;
        }
        bare |= name.starts_with("double_blocks.") || name.starts_with("single_blocks.");
        prefixed |= name.starts_with("model.diffusion_model.");
        if bare && prefixed {
            break;
        }
    }
    bare && !prefixed
}

const STRING: u32 = 8;
const ARRAY: u32 = 9;

fn skip_tensor_info(input: &mut impl Read) -> bool {
    let Some(dims) = read_u32(input) else {
        return false;
    };
    for _ in 0..dims {
        if read_u64(input).is_none() {
            return false;
        }
    }
    read_u32(input).is_some() && read_u64(input).is_some()
}

fn skip_value(input: &mut impl Read, kind: u32) -> bool {
    match kind {
        0 | 1 | 7 => skip_bytes(input, 1),
        2 | 3 => skip_bytes(input, 2),
        4..=6 => skip_bytes(input, 4),
        STRING => read_string(input).is_some(),
        ARRAY => skip_array(input),
        10..=12 => skip_bytes(input, 8),
        _ => false,
    }
}

fn skip_array(input: &mut impl Read) -> bool {
    let (Some(kind), Some(count)) = (read_u32(input), read_u64(input)) else {
        return false;
    };
    if count > 1_000_000 {
        return false;
    }
    for _ in 0..count {
        if !skip_value(input, kind) {
            return false;
        }
    }
    true
}

fn skip_bytes(input: &mut impl Read, size: usize) -> bool {
    let mut buf = [0; 8];
    input.read_exact(&mut buf[..size]).is_ok()
}

fn read_string(input: &mut impl Read) -> Option<String> {
    let len = read_u64(input)?;
    if len > 1_048_576 {
        return None;
    }
    let len = usize::try_from(len).ok()?;
    let mut buf = vec![0; len];
    input.read_exact(&mut buf).ok()?;
    String::from_utf8(buf).ok()
}

fn read_u32(input: &mut impl Read) -> Option<u32> {
    let mut buf = [0; 4];
    input.read_exact(&mut buf).ok()?;
    Some(u32::from_le_bytes(buf))
}

fn read_u64(input: &mut impl Read) -> Option<u64> {
    let mut buf = [0; 8];
    input.read_exact(&mut buf).ok()?;
    Some(u64::from_le_bytes(buf))
}
