//! Escolhe os flags do `sd-cli` a partir da pasta ou do arquivo escolhido.
//!
//! Um checkpoint completo segue em `-m`. Um Flux em `transformer/` ou um GGUF só do
//! diffusion vai em `--diffusion-model`, com o CLIP, o T5, o LLM e o VAE ao lado.

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

/// Partes que o `sd-cli` abre em separado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelParts {
    /// Checkpoint completo, no `-m`.
    pub checkpoint: Option<PathBuf>,
    /// Transformer ou `UNet` solto, no `--diffusion-model`.
    pub diffusion: Option<PathBuf>,
    /// CLIP-L, no `--clip_l`.
    pub clip_l: Option<PathBuf>,
    /// T5-XXL, no `--t5xxl`.
    pub t5xxl: Option<PathBuf>,
    /// Codificador de texto LLM, no `--llm`.
    pub llm: Option<PathBuf>,
    /// `tokenizer.json` exigido por alguns LLMs, no `--tokenizer`.
    pub tokenizer: Option<PathBuf>,
    /// VAE, no `--vae`.
    pub vae: Option<PathBuf>,
}

/// Acrescenta os flags do modelo. Um arquivo desconhecido continua no `-m`.
pub fn extend_model(args: &mut Vec<String>, model: &Path) {
    let mut parts = parts_for(model);
    if parts.checkpoint.is_none() && parts.diffusion.is_none() {
        parts.checkpoint = Some(model.to_path_buf());
    }
    let offload = parts.diffusion.as_ref().is_some_and(|path| {
        path.extension()
            .and_then(|ext| ext.to_str())
            .is_none_or(|ext| !ext.eq_ignore_ascii_case("gguf"))
    });
    push(&mut parts.checkpoint, args, "-m");
    push(&mut parts.diffusion, args, "--diffusion-model");
    push(&mut parts.clip_l, args, "--clip_l");
    push(&mut parts.t5xxl, args, "--t5xxl");
    push(&mut parts.llm, args, "--llm");
    push(&mut parts.tokenizer, args, "--tokenizer");
    push(&mut parts.vae, args, "--vae");
    if offload {
        args.push("--offload-to-cpu".to_owned());
    }
}

/// Backend de execução. O LLM fica na CPU: uma camada do Qwen não cabe na VRAM.
#[must_use]
pub fn runtime_backend(model: &Path, device: &str) -> String {
    if parts_for(model).llm.is_some() {
        format!("llm=cpu,{device}")
    } else {
        device.to_owned()
    }
}

fn push(path: &mut Option<PathBuf>, args: &mut Vec<String>, flag: &str) {
    let Some(path) = path.take() else {
        return;
    };
    args.push(flag.to_owned());
    args.push(path.display().to_string());
}

fn parts_for(path: &Path) -> ModelParts {
    if path.is_file() && bare_flux_gguf(path) {
        let mut parts = path.parent().map_or_else(ModelParts::empty, components_in);
        parts.checkpoint = None;
        parts.diffusion = Some(path.to_path_buf());
        return parts;
    }
    if path.is_dir() && !has_unet(path) && (has_transformer(path) || has_index(path)) {
        return components_in(path);
    }
    ModelParts {
        checkpoint: Some(path.to_path_buf()),
        ..ModelParts::empty()
    }
}

impl ModelParts {
    fn empty() -> Self {
        Self {
            checkpoint: None,
            diffusion: None,
            clip_l: None,
            t5xxl: None,
            llm: None,
            tokenizer: None,
            vae: None,
        }
    }
}

fn components_in(root: &Path) -> ModelParts {
    let index = read_index(root);
    let mut parts = ModelParts {
        diffusion: component_weight(&root.join("transformer")),
        vae: component_weight(&root.join("vae")),
        ..ModelParts::empty()
    };
    place_first(
        index
            .as_ref()
            .and_then(|value| component_class(value, "text_encoder")),
        component_weight(&root.join("text_encoder")),
        &mut parts,
    );
    place_second(
        index
            .as_ref()
            .and_then(|value| component_class(value, "text_encoder_2")),
        component_weight(&root.join("text_encoder_2")),
        &mut parts,
    );
    if parts.llm.is_some() {
        parts.tokenizer = tokenizer_file(root);
    }
    parts
}

fn tokenizer_file(root: &Path) -> Option<PathBuf> {
    let nested = root.join("tokenizer").join("tokenizer.json");
    if nested.is_file() {
        return Some(nested);
    }
    let flat = root.join("tokenizer.json");
    flat.is_file().then_some(flat)
}

fn place_first(class_name: Option<&str>, file: Option<PathBuf>, parts: &mut ModelParts) {
    if class_name.is_some_and(is_llm) {
        parts.llm = file;
    } else {
        parts.clip_l = file;
    }
}

fn place_second(class_name: Option<&str>, file: Option<PathBuf>, parts: &mut ModelParts) {
    if class_name.is_some_and(is_llm) {
        parts.llm = file;
    } else if class_name.is_none_or(is_t5) {
        parts.t5xxl = file;
    }
}

fn is_t5(class_name: &str) -> bool {
    class_name.to_ascii_lowercase().contains("t5")
}

fn is_clip(class_name: &str) -> bool {
    class_name.to_ascii_lowercase().contains("clip")
}

fn is_llm(class_name: &str) -> bool {
    !is_t5(class_name) && !is_clip(class_name)
}

fn component_class<'a>(index: &'a serde_json::Value, key: &str) -> Option<&'a str> {
    index.get(key)?.as_array()?.last()?.as_str()
}

fn read_index(root: &Path) -> Option<serde_json::Value> {
    let file = File::open(root.join("model_index.json")).ok()?;
    serde_json::from_reader(file).ok()
}

fn has_unet(dir: &Path) -> bool {
    weight_named(dir, "unet", "diffusion_pytorch_model").is_some()
}

fn has_transformer(dir: &Path) -> bool {
    component_weight(&dir.join("transformer")).is_some()
}

fn has_index(dir: &Path) -> bool {
    dir.join("model_index.json").is_file()
}

fn weight_named(root: &Path, folder: &str, stem: &str) -> Option<PathBuf> {
    let dir = root.join(folder);
    let index = dir.join(format!("{stem}.safetensors.index.json"));
    if index.is_file() {
        return Some(index);
    }
    let single = dir.join(format!("{stem}.safetensors"));
    single.is_file().then_some(single)
}

fn component_weight(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut index = None;
    let mut single = None;
    for entry in entries.flatten() {
        let Some(name) = entry.file_name().into_string().ok() else {
            continue;
        };
        if name.ends_with(".safetensors.index.json") {
            index = Some(entry.path());
        } else if name.ends_with(".safetensors") && !name.contains("-of-") {
            single = Some(entry.path());
        }
    }
    index.or(single)
}

fn bare_flux_gguf(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"))
        && gguf_is_bare_flux(path)
}

fn gguf_is_bare_flux(path: &Path) -> bool {
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
    let mut architecture = String::new();
    for _ in 0..pairs {
        let Some(key) = read_string(&mut input) else {
            return false;
        };
        let Some(kind) = read_u32(&mut input) else {
            return false;
        };
        if key == "general.architecture" && kind == STRING {
            architecture = read_string(&mut input).unwrap_or_default();
        } else if !skip_value(&mut input, kind) {
            return false;
        }
    }
    if architecture != "flux" {
        return false;
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

#[cfg(test)]
mod tests {
    use super::{bare_flux_gguf, extend_model, parts_for, runtime_backend};

    fn touch(path: &std::path::Path) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, []).unwrap();
    }

    fn flux_tree(root: &std::path::Path) {
        let index = r#"{
            "_class_name": "FluxPipeline",
            "text_encoder": ["transformers", "CLIPTextModel"],
            "text_encoder_2": ["transformers", "T5EncoderModel"],
            "transformer": ["diffusers", "FluxTransformer2DModel"],
            "vae": ["diffusers", "AutoencoderKL"]
        }"#;
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join("model_index.json"), index).unwrap();
        touch(&root.join("transformer/diffusion_pytorch_model.safetensors.index.json"));
        touch(&root.join("text_encoder/model.safetensors"));
        touch(&root.join("text_encoder_2/model.safetensors.index.json"));
        touch(&root.join("vae/diffusion_pytorch_model.safetensors"));
    }

    fn write_flux_gguf(path: &std::path::Path) {
        let mut data = Vec::new();
        data.extend(b"GGUF");
        data.extend(&3u32.to_le_bytes());
        data.extend(&1u64.to_le_bytes());
        data.extend(&3u64.to_le_bytes());
        push_str(&mut data, "general.architecture");
        data.extend(&8u32.to_le_bytes());
        push_str(&mut data, "flux");
        push_str(&mut data, "general.quantization_version");
        data.extend(&4u32.to_le_bytes());
        data.extend(&2u32.to_le_bytes());
        push_str(&mut data, "general.file_type");
        data.extend(&4u32.to_le_bytes());
        data.extend(&15u32.to_le_bytes());
        push_str(&mut data, "double_blocks.0.img_attn.qkv.weight");
        data.extend(&1u32.to_le_bytes());
        data.extend(&1u64.to_le_bytes());
        data.extend(&0u32.to_le_bytes());
        data.extend(&0u64.to_le_bytes());
        std::fs::write(path, data).unwrap();
    }

    fn push_str(data: &mut Vec<u8>, text: &str) {
        let len = u64::try_from(text.len()).unwrap();
        data.extend(&len.to_le_bytes());
        data.extend(text.as_bytes());
    }

    #[test]
    fn a_pasta_flux_abre_cada_parte() {
        let root = std::env::temp_dir().join(format!("voxel-flux-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        flux_tree(&root);
        let parts = parts_for(&root);
        assert!(parts.checkpoint.is_none());
        assert_eq!(
            parts.diffusion.unwrap().file_name().unwrap(),
            "diffusion_pytorch_model.safetensors.index.json"
        );
        assert!(
            parts
                .clip_l
                .unwrap()
                .ends_with("text_encoder/model.safetensors")
        );
        assert!(
            parts
                .t5xxl
                .unwrap()
                .ends_with("text_encoder_2/model.safetensors.index.json")
        );
        assert!(
            parts
                .vae
                .unwrap()
                .ends_with("vae/diffusion_pytorch_model.safetensors")
        );
        assert!(parts.llm.is_none());
        let mut args = Vec::new();
        extend_model(&mut args, &root);
        assert!(!args.iter().any(|arg| arg == "-m"));
        assert!(args.iter().any(|arg| arg == "--diffusion-model"));
        assert!(args.iter().any(|arg| arg == "--clip_l"));
        assert!(args.iter().any(|arg| arg == "--t5xxl"));
        assert!(args.iter().any(|arg| arg == "--vae"));
        assert!(args.iter().any(|arg| arg == "--offload-to-cpu"));
        assert!(!args.iter().any(|arg| arg == "--tokenizer"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn o_gguf_flux_usa_o_arquivo_como_diffusion_e_os_vizinhos() {
        let root = std::env::temp_dir().join(format!("voxel-gguf-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        flux_tree(&root);
        let gguf = root.join("peso-Q4_K_M.gguf");
        write_flux_gguf(&gguf);
        let parts = parts_for(&gguf);
        assert_eq!(parts.diffusion.unwrap(), gguf);
        assert!(parts.clip_l.is_some());
        assert!(parts.t5xxl.is_some());
        assert!(parts.vae.is_some());
        assert!(parts.checkpoint.is_none());
        let mut args = Vec::new();
        extend_model(&mut args, &gguf);
        assert!(args.iter().any(|arg| arg == "--diffusion-model"));
        assert!(!args.iter().any(|arg| arg == "--offload-to-cpu"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn o_qwen_vai_no_llm() {
        let root = std::env::temp_dir().join(format!("voxel-moss-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("model_index.json"),
            r#"{"text_encoder":["transformers","Qwen3_5Model"]}"#,
        )
        .unwrap();
        touch(&root.join("text_encoder/model.safetensors.index.json"));
        touch(&root.join("transformer/diffusion_pytorch_model.safetensors.index.json"));
        touch(&root.join("vae/diffusion_pytorch_model.safetensors"));
        touch(&root.join("tokenizer/tokenizer.json"));
        let parts = parts_for(&root);
        assert!(
            parts
                .llm
                .unwrap()
                .ends_with("text_encoder/model.safetensors.index.json")
        );
        assert!(
            parts
                .tokenizer
                .unwrap()
                .ends_with("tokenizer/tokenizer.json")
        );
        assert!(parts.clip_l.is_none());
        assert!(parts.t5xxl.is_none());
        let mut args = Vec::new();
        extend_model(&mut args, &root);
        assert!(
            args.windows(2)
                .any(|pair| pair[0] == "--tokenizer"
                    && pair[1].ends_with("tokenizer/tokenizer.json"))
        );
        assert_eq!(runtime_backend(&root, "Vulkan0"), "llm=cpu,Vulkan0");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_pasta_com_unet_continua_no_checkpoint() {
        let root = std::env::temp_dir().join(format!("voxel-unet-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        touch(&root.join("unet/diffusion_pytorch_model.safetensors"));
        touch(&root.join("model_index.json"));
        let parts = parts_for(&root);
        assert_eq!(parts.checkpoint.unwrap(), root);
        assert!(parts.diffusion.is_none());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn o_gguf_da_variavel_e_diffusion_solto() {
        let Ok(path) = std::env::var("VOXEL_FLUX_GGUF") else {
            return;
        };
        let path = std::path::PathBuf::from(path);
        if !path.is_file() {
            return;
        }
        assert!(bare_flux_gguf(&path));
        let parts = parts_for(&path);
        assert_eq!(parts.diffusion.as_deref(), Some(path.as_path()));
        assert!(parts.clip_l.is_some());
        assert!(parts.t5xxl.is_some());
        assert!(parts.vae.is_some());
    }

    #[test]
    fn um_arquivo_comum_continua_no_checkpoint() {
        let root = std::env::temp_dir().join(format!("voxel-sd15-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let file = root.join("sd15.safetensors");
        touch(&file);
        let parts = parts_for(&file);
        assert_eq!(parts.checkpoint.unwrap(), file);
        assert_eq!(runtime_backend(&file, "Vulkan0"), "Vulkan0");
        let _ = std::fs::remove_dir_all(&root);
    }
}
