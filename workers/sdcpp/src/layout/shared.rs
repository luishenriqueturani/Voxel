//! Empresta o VAE e os codificadores de texto de outro pacote instalado no Hub.
//!
//! Todo Flux.1 usa o mesmo VAE, o CLIP-L e o T5. O Qwen Image usa o próprio VAE e o
//! LLM. Um arquivo só do transformer roda com as peças da família correspondente.

use std::path::{Path, PathBuf};

use super::{ModelParts, components_in, is_qwen_image, read_index};

pub fn borrow_missing(model: &Path, parts: &mut ModelParts) {
    let Some(hub) = model
        .ancestors()
        .find(|dir| dir.file_name() == Some("hub".as_ref()))
    else {
        return;
    };
    if is_qwen_image(model) {
        if parts.vae.is_some() && parts.llm.is_some() {
            return;
        }
        fill(hub, parts, is_qwen, |parts, found| {
            parts.vae = parts.vae.take().or(found.vae);
            parts.llm = parts.llm.take().or(found.llm);
            parts.tokenizer = parts.tokenizer.take().or(found.tokenizer);
            parts.vae.is_some() && parts.llm.is_some()
        });
        return;
    }
    if parts.vae.is_some() && parts.clip_l.is_some() && parts.t5xxl.is_some() {
        return;
    }
    fill(hub, parts, is_flux1, |parts, found| {
        parts.vae = parts.vae.take().or(found.vae);
        parts.clip_l = parts.clip_l.take().or(found.clip_l);
        parts.t5xxl = parts.t5xxl.take().or(found.t5xxl);
        parts.vae.is_some() && parts.clip_l.is_some() && parts.t5xxl.is_some()
    });
}

fn fill(
    hub: &Path,
    parts: &mut ModelParts,
    keep: fn(&Path) -> bool,
    take: fn(&mut ModelParts, ModelParts) -> bool,
) {
    for package in packages(hub, keep) {
        if take(parts, components_in(&package)) {
            return;
        }
    }
}

fn packages(hub: &Path, keep: fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = subdirs(hub)
        .iter()
        .flat_map(|owner| subdirs(owner))
        .flat_map(|repo| {
            let mut candidates = subdirs(&repo);
            candidates.insert(0, repo);
            candidates
        })
        .filter(|dir| keep(dir))
        .collect();
    found.sort();
    found
}

fn is_flux1(dir: &Path) -> bool {
    read_index(dir).is_some_and(|index| {
        index.get("_class_name").and_then(serde_json::Value::as_str) == Some("FluxPipeline")
    })
}

fn is_qwen(dir: &Path) -> bool {
    if dir.join("text_encoders").is_dir() && dir.join("vae").is_dir() {
        return true;
    }
    read_index(dir).is_some_and(|index| {
        index
            .get("_class_name")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|class| class.to_ascii_lowercase().contains("qwen"))
    })
}

/// Pasta Diffusers do Qwen Image 2.1 já instalada, para um adaptador usar de base.
#[must_use]
pub fn find_qwen_base(lora: &Path) -> Option<PathBuf> {
    let hub = lora
        .ancestors()
        .find(|dir| dir.file_name() == Some("hub".as_ref()))?;
    let mut found = packages(hub, is_qwen_pipeline);
    found.sort_by_key(|path| {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        (u8::from(name.contains("turbo")), name)
    });
    found.into_iter().next()
}

/// Acrescenta a marca do adaptador no prompt quando o arquivo escolhido é um `LoRA`.
#[must_use]
pub fn prompt_with_lora(model: &Path, prompt: &str) -> String {
    if !super::is_lora(model) {
        return prompt.to_owned();
    }
    format!("<lora:{}:1> {prompt}", model.display())
}

/// Pasta do adaptador para o `sd-cli` achar o arquivo citado no prompt.
pub fn extend_lora(args: &mut Vec<String>, model: &Path) {
    if !super::is_lora(model) {
        return;
    }
    let Some(dir) = model.parent() else {
        return;
    };
    args.push("--lora-model-dir".to_owned());
    args.push(dir.display().to_string());
}

fn is_qwen_pipeline(dir: &Path) -> bool {
    read_index(dir).is_some_and(|index| {
        index
            .get("_class_name")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|class| class.contains("QwenImage"))
    })
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect()
}
