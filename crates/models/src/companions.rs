//! Junta ao arquivo só do transformer as peças que ele precisa para gerar.
//!
//! Um GGUF ou safetensors na raiz de um repositório Diffusers costuma trazer só o
//! transformer. O VAE e os codificadores de texto ficam nas pastas do mesmo repositório,
//! e entram no pacote do arquivo para o download trazer tudo de uma vez.

use crate::package::{HubPackage, HubPart};

/// O pacote já inclui o VAE e os codificadores de texto do mesmo repositório.
pub const INCLUDED: &str = "included";
/// O arquivo é um Flux.1 só do transformer, sem as peças no repositório.
pub const BORROWED: &str = "borrowed";
/// O arquivo é um `LoRA` e precisa de um modelo-base.
pub const LORA: &str = "lora";

const SHARED: &[&str] = &[
    "vae",
    "text_encoder",
    "text_encoder_2",
    "text_encoders",
    "tokenizer",
    "tokenizer_2",
    "processor",
    "model_index.json",
];

const MIN_TRANSFORMER: u64 = 1024 * 1024 * 1024;

/// Acrescenta as peças compartilhadas a cada arquivo solto que é só o transformer.
pub fn attach(packages: &mut [HubPackage]) {
    let extras: Vec<Option<Vec<HubPart>>> = packages
        .iter()
        .map(|package| companions_for(package, packages))
        .collect();
    for (package, extra) in packages.iter_mut().zip(extras) {
        if let Some(parts) = extra {
            package.parts.extend(parts);
            INCLUDED.clone_into(&mut package.extras);
        }
    }
}

/// Marca os GGUF soltos de um repositório Flux.1 sem as peças compartilhadas.
pub fn mark_borrowed(packages: &mut [HubPackage]) {
    for package in packages.iter_mut() {
        if package.extras.is_empty()
            && !package.directory
            && is_gguf(&package.opener)
            && !package.opener.to_ascii_lowercase().contains("qwen")
        {
            BORROWED.clone_into(&mut package.extras);
        }
    }
}

/// Marca os arquivos cujo nome indica um `LoRA`.
pub fn mark_loras(packages: &mut [HubPackage]) {
    for package in packages.iter_mut() {
        if !package.directory && is_lora_name(&package.opener) {
            LORA.clone_into(&mut package.extras);
        }
    }
}

fn companions_for(package: &HubPackage, all: &[HubPackage]) -> Option<Vec<HubPart>> {
    if package.directory || !is_weight(&package.opener) || is_lora_name(&package.opener) {
        return None;
    }
    let bytes: u64 = package.parts.iter().map(|part| part.bytes).sum();
    if bytes < MIN_TRANSFORMER {
        return None;
    }
    let folder = package.opener.rsplit_once('/').map_or("", |(dir, _)| dir);
    let mut extra = Vec::new();
    for other in all {
        if other.repo != package.repo {
            continue;
        }
        for part in &other.parts {
            if part.filename == package.opener {
                continue;
            }
            if extra
                .iter()
                .any(|seen: &HubPart| seen.filename == part.filename)
            {
                continue;
            }
            if SHARED.contains(&component(&part.filename, folder)) {
                extra.push(part.clone());
            }
        }
    }
    (!extra.is_empty()).then_some(extra)
}

fn is_lora_name(name: &str) -> bool {
    name.to_ascii_lowercase().contains("lora")
}

fn component<'a>(filename: &'a str, folder: &str) -> &'a str {
    let inside = if folder.is_empty() {
        filename
    } else {
        filename
            .strip_prefix(folder)
            .and_then(|rest| rest.strip_prefix('/'))
            .unwrap_or("")
    };
    inside.split('/').next().unwrap_or("")
}

fn is_weight(name: &str) -> bool {
    is_gguf(name) || has_extension(name, "safetensors")
}

fn is_gguf(name: &str) -> bool {
    has_extension(name, "gguf")
}

fn has_extension(name: &str, wanted: &str) -> bool {
    std::path::Path::new(name)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case(wanted))
}

#[cfg(test)]
mod tests {
    use super::{BORROWED, INCLUDED, mark_borrowed};
    use crate::hub::HubFile;
    use crate::package::group_packages;

    const GIB: u64 = 1024 * 1024 * 1024;

    fn file(filename: &str, bytes: u64) -> HubFile {
        HubFile {
            repo: "acme/flux".to_owned(),
            filename: filename.to_owned(),
            bytes,
            sha256: String::new(),
            license: "other".to_owned(),
            gated: false,
        }
    }

    #[test]
    fn o_gguf_da_raiz_leva_o_vae_e_os_codificadores() {
        let packages = group_packages(&[
            file("model_index.json", 1),
            file("transformer/diffusion_pytorch_model.safetensors", 20 * GIB),
            file("vae/diffusion_pytorch_model.safetensors", 160),
            file("text_encoder/model.safetensors", 240),
            file("text_encoder_2/model.safetensors.index.json", 2),
            file("text_encoder_2/model-00001-of-00002.safetensors", 5 * GIB),
            file("text_encoder_2/model-00002-of-00002.safetensors", 4 * GIB),
            file("tokenizer_2/tokenizer.json", 2),
            file("peso-Q8_0.gguf", 12 * GIB),
            file("lora.safetensors", 100),
        ]);
        let gguf = packages
            .iter()
            .find(|package| package.opener == "peso-Q8_0.gguf")
            .unwrap();
        assert_eq!(gguf.extras, INCLUDED);
        let names: Vec<&str> = gguf
            .parts
            .iter()
            .map(|part| part.filename.as_str())
            .collect();
        assert_eq!(names[0], "peso-Q8_0.gguf");
        assert!(names.contains(&"vae/diffusion_pytorch_model.safetensors"));
        assert!(names.contains(&"text_encoder_2/model-00002-of-00002.safetensors"));
        assert!(names.contains(&"tokenizer_2/tokenizer.json"));
        assert!(names.contains(&"model_index.json"));
        assert!(!names.iter().any(|name| name.starts_with("transformer/")));
        let lora = packages
            .iter()
            .find(|package| package.opener == "lora.safetensors")
            .unwrap();
        assert_eq!(lora.extras, super::LORA);
        assert_eq!(lora.parts.len(), 1);
    }

    #[test]
    fn o_gguf_qwen_leva_o_vae_e_o_llm_do_mesmo_repositorio() {
        let packages = group_packages(&[
            file("qwen-image-2.1-UC-BF16.gguf", 12 * GIB),
            file("vae/qwen_image_2.1_vae_bf16.safetensors", 644),
            file("text_encoders/qwen3vl_8b_bf16.safetensors", 16 * GIB),
            file("omni-lora-v0.1-bf16.safetensors", 640),
        ]);
        let gguf = packages
            .iter()
            .find(|package| package.opener == "qwen-image-2.1-UC-BF16.gguf")
            .unwrap();
        assert_eq!(gguf.extras, INCLUDED);
        let names: Vec<&str> = gguf
            .parts
            .iter()
            .map(|part| part.filename.as_str())
            .collect();
        assert!(names.contains(&"vae/qwen_image_2.1_vae_bf16.safetensors"));
        assert!(names.contains(&"text_encoders/qwen3vl_8b_bf16.safetensors"));
        let lora = packages
            .iter()
            .find(|package| package.opener.ends_with("lora-v0.1-bf16.safetensors"))
            .unwrap();
        assert_eq!(lora.extras, super::LORA);
        assert_eq!(lora.parts.len(), 1);
    }

    #[test]
    fn sem_pecas_compartilhadas_o_gguf_flux_fica_marcado_como_emprestado() {
        let mut packages = group_packages(&[file("flux-Q4_K_M.gguf", 6 * GIB)]);
        assert!(packages.iter().all(|package| package.extras.is_empty()));
        mark_borrowed(&mut packages);
        let gguf = packages
            .iter()
            .find(|package| package.opener == "flux-Q4_K_M.gguf")
            .unwrap();
        assert_eq!(gguf.extras, BORROWED);
        assert_eq!(gguf.parts.len(), 1);
    }
}
