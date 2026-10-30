//! Escolhe os flags do `sd-cli` a partir da pasta ou do arquivo escolhido.
//!
//! Um checkpoint completo segue em `-m`. Um Flux em `transformer/` ou um arquivo só do
//! transformer vai em `--diffusion-model`, com o CLIP, o T5, o LLM e o VAE ao lado.

mod header;
mod shared;
#[cfg(test)]
mod tests;

use std::fs::File;
use std::path::{Path, PathBuf};

pub use header::{bare_diffusion, is_lora, is_qwen_image};
pub use shared::{extend_lora, find_qwen_base, prompt_with_lora};

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

/// Partes que um arquivo só do transformer não achou. Vazio quando nada falta.
#[must_use]
pub fn missing_parts(model: &Path) -> Vec<&'static str> {
    if is_lora(model) {
        return if find_qwen_base(model).is_some() {
            Vec::new()
        } else {
            vec!["modelo-base"]
        };
    }
    if !bare_diffusion(model) {
        return Vec::new();
    }
    let parts = parts_for(model);
    let needed = if is_qwen_image(model) {
        vec![("VAE", parts.vae.is_none()), ("LLM", parts.llm.is_none())]
    } else {
        vec![
            ("VAE", parts.vae.is_none()),
            ("CLIP-L", parts.clip_l.is_none()),
            ("T5-XXL", parts.t5xxl.is_none()),
        ]
    };
    needed
        .into_iter()
        .filter_map(|(name, missing)| missing.then_some(name))
        .collect()
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

/// Tipo dos tensores. O Vulkan RADV não calcula BF16; o Qwen Image 2.1 Diffusers é BF16.
#[must_use]
pub fn tensor_type_rules(model: &Path, device: &str) -> &'static str {
    if uses_gguf(model) || !device.to_ascii_lowercase().contains("vulkan") {
        r"^vae\.=f16"
    } else {
        r"^model\.=f16,^vae\.=f16,^text_encoders\.=f16"
    }
}

/// Prefetch e orçamento de VRAM para o transformer do Qwen, que não cabe inteiro na GPU.
pub fn extend_runtime_flags(args: &mut Vec<String>, model: &Path) {
    if parts_for(model).llm.is_none() {
        return;
    }
    args.push("--disable-prefetch".to_owned());
    args.push("--max-vram".to_owned());
    args.push("-2".to_owned());
}

fn uses_gguf(model: &Path) -> bool {
    let parts = parts_for(model);
    parts
        .diffusion
        .as_deref()
        .or(parts.checkpoint.as_deref())
        .is_some_and(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("gguf"))
        })
}

fn push(path: &mut Option<PathBuf>, args: &mut Vec<String>, flag: &str) {
    let Some(path) = path.take() else {
        return;
    };
    args.push(flag.to_owned());
    args.push(path.display().to_string());
}

fn parts_for(path: &Path) -> ModelParts {
    if is_lora(path) {
        if let Some(base) = find_qwen_base(path) {
            return parts_for(&base);
        }
        return ModelParts {
            checkpoint: Some(path.to_path_buf()),
            ..ModelParts::empty()
        };
    }
    if bare_diffusion(path) {
        let mut parts = path.parent().map_or_else(ModelParts::empty, components_in);
        parts.checkpoint = None;
        parts.diffusion = Some(path.to_path_buf());
        shared::borrow_missing(path, &mut parts);
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
    if parts.llm.is_none() {
        parts.llm = component_weight(&root.join("text_encoders"));
    }
    if parts.llm.is_some() {
        parts.tokenizer = tokenizer_file(root);
    }
    parts
}

fn tokenizer_file(root: &Path) -> Option<PathBuf> {
    [
        root.join("tokenizer").join("tokenizer.json"),
        root.join("processor").join("tokenizer.json"),
        root.join("tokenizer.json"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_file())
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
