use std::path::Path;

use voxel_protocol::{Img2ImgRequest, InpaintRequest, Txt2ImgRequest};

/// Parâmetros comuns de uma passagem do `sd-cli`.
pub struct ImageJob {
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub steps: i64,
    pub width: i64,
    pub height: i64,
    pub seed: i64,
    pub cfg_scale: f64,
    pub init_image: Option<String>,
    pub strength: Option<f64>,
    pub mask: Option<String>,
    pub circular: bool,
}

/// Configuração de decode usada quando o processo morre depois de gravar o latente.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeFallback {
    pub tiling: bool,
    pub vae: &'static str,
    pub cpu: bool,
}

/// Tiles, depois fp32, depois o VAE na CPU. Cada passo é um processo novo.
pub const DECODE_FALLBACKS: &[DecodeFallback] = &[
    DecodeFallback {
        tiling: true,
        vae: "f16",
        cpu: false,
    },
    DecodeFallback {
        tiling: true,
        vae: "f32",
        cpu: false,
    },
    DecodeFallback {
        tiling: false,
        vae: "f32",
        cpu: true,
    },
];

/// Último `passo/total` impresso pelo `sd-cli`.
#[must_use]
pub fn parse_progress(text: &str) -> Option<(i64, i64)> {
    let mut found = None;
    for token in text.split_whitespace() {
        let Some((step, total)) = token.split_once('/') else {
            continue;
        };
        let (Ok(step), Ok(total)) = (step.parse::<i64>(), total.parse::<i64>()) else {
            continue;
        };
        if total > 0 && (0..=total).contains(&step) {
            found = Some((step, total));
        }
    }
    found
}

/// Nome do dispositivo cujo prefixo corresponde ao backend.
#[must_use]
pub fn device_for(backend: &str, devices: &[String]) -> Option<String> {
    let prefix = backend.to_ascii_lowercase();
    devices
        .iter()
        .find(|device| device.to_ascii_lowercase().starts_with(&prefix))
        .cloned()
}

/// Linhas `nome<TAB>descrição` do `--list-devices`.
#[must_use]
pub fn parse_devices(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let (name, _) = line.split_once('\t')?;
            let name = name.trim();
            (!name.is_empty()).then(|| name.to_owned())
        })
        .collect()
}

/// Argumentos do txt2img. A prévia usa a projeção, que não precisa de outro modelo.
#[must_use]
pub fn txt2img_args(
    model: &Path,
    output: &Path,
    preview: &Path,
    device: &str,
    request: &Txt2ImgRequest,
) -> Vec<String> {
    image_args(
        model,
        output,
        preview,
        device,
        &ImageJob {
            prompt: request.prompt.clone(),
            negative_prompt: request.negative_prompt.clone(),
            steps: request.steps,
            width: request.width,
            height: request.height,
            seed: request.seed,
            cfg_scale: request.cfg_scale.unwrap_or(7.0),
            init_image: None,
            strength: None,
            mask: None,
            circular: request.circular.unwrap_or(false),
        },
    )
}

/// Argumentos do img2img. A intensidade diz quanto da imagem de origem pode mudar.
#[must_use]
pub fn img2img_args(
    model: &Path,
    output: &Path,
    preview: &Path,
    device: &str,
    request: &Img2ImgRequest,
) -> Vec<String> {
    image_args(
        model,
        output,
        preview,
        device,
        &ImageJob {
            prompt: request.prompt.clone(),
            negative_prompt: request.negative_prompt.clone(),
            steps: request.steps,
            width: request.width.unwrap_or(512),
            height: request.height.unwrap_or(512),
            seed: request.seed.unwrap_or(1),
            cfg_scale: request.cfg_scale.unwrap_or(7.0),
            init_image: Some(request.image_path.clone()),
            strength: Some(request.strength),
            mask: None,
            circular: request.circular.unwrap_or(false),
        },
    )
}

/// Argumentos do inpainting. A máscara branca é a área recriada.
#[must_use]
pub fn inpaint_args(
    model: &Path,
    output: &Path,
    preview: &Path,
    device: &str,
    request: &InpaintRequest,
) -> Vec<String> {
    image_args(
        model,
        output,
        preview,
        device,
        &ImageJob {
            prompt: request.prompt.clone(),
            negative_prompt: request.negative_prompt.clone(),
            steps: request.steps,
            width: request.width.unwrap_or(512),
            height: request.height.unwrap_or(512),
            seed: request.seed.unwrap_or(1),
            cfg_scale: request.cfg_scale.unwrap_or(7.0),
            init_image: Some(request.image_path.clone()),
            strength: Some(request.strength),
            mask: Some(request.mask_path.clone()),
            circular: request.circular.unwrap_or(false),
        },
    )
}

/// Linha de comando compartilhada pelas três passagens de imagem.
#[must_use]
pub fn image_args(
    model: &Path,
    output: &Path,
    preview: &Path,
    device: &str,
    job: &ImageJob,
) -> Vec<String> {
    let mut args = vec![
        "-m".to_owned(),
        model.display().to_string(),
        "-p".to_owned(),
        job.prompt.clone(),
        "-o".to_owned(),
        output.display().to_string(),
        "--preview".to_owned(),
        "proj".to_owned(),
        "--preview-path".to_owned(),
        preview.display().to_string(),
        "--preview-interval".to_owned(),
        "1".to_owned(),
        "--steps".to_owned(),
        job.steps.to_string(),
        "-W".to_owned(),
        job.width.to_string(),
        "-H".to_owned(),
        job.height.to_string(),
        "--seed".to_owned(),
        job.seed.to_string(),
        "--cfg-scale".to_owned(),
        job.cfg_scale.to_string(),
        "--sampling-method".to_owned(),
        "euler_a".to_owned(),
        "--backend".to_owned(),
        device.to_owned(),
        "--tensor-type-rules".to_owned(),
        r"^vae\.=f16".to_owned(),
        "--auto-fit".to_owned(),
        "on".to_owned(),
    ];
    if let Some(negative) = &job.negative_prompt {
        args.push("-n".to_owned());
        args.push(negative.clone());
    }
    if let Some(path) = &job.init_image {
        args.push("--init-img".to_owned());
        args.push(path.clone());
    }
    if let Some(strength) = job.strength {
        args.push("--strength".to_owned());
        args.push(format_strength(strength));
    }
    if let Some(path) = &job.mask {
        args.push("--mask".to_owned());
        args.push(path.clone());
    }
    if job.circular {
        args.push("--circular".to_owned());
    }
    args
}

fn format_strength(value: f64) -> String {
    let clamped = if value.is_finite() {
        value.clamp(0.05, 1.0)
    } else {
        0.75
    };
    format!("{clamped:.2}")
}

/// Decode de um latente já gravado, sem refazer o denoising.
#[must_use]
pub fn decode_args(
    model: &Path,
    output: &Path,
    device: &str,
    fallback: DecodeFallback,
) -> Vec<String> {
    let mut args = vec![
        "-m".to_owned(),
        model.display().to_string(),
        "-o".to_owned(),
        output.display().to_string(),
        "--backend".to_owned(),
        if fallback.cpu {
            format!("vae=cpu,{device}")
        } else {
            device.to_owned()
        },
        "--tensor-type-rules".to_owned(),
        format!("^vae\\.={}", fallback.vae),
    ];
    if fallback.tiling {
        args.push("--vae-tiling".to_owned());
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_o_passo_no_meio_da_barra_de_progresso() {
        let line = "\r|==== 2/4 - 1.50it/s\u{1b}[K";
        assert_eq!(parse_progress(line), Some((2, 4)));
        assert_eq!(parse_progress("sem passo"), None);
    }

    #[test]
    fn escolhe_o_dispositivo_pelo_prefixo() {
        let devices = parse_devices("Vulkan0\tGPU\nCPU\tCPU\n");
        assert_eq!(device_for("vulkan", &devices).as_deref(), Some("Vulkan0"));
        assert_eq!(device_for("cuda", &devices), None);
    }

    #[test]
    fn o_decode_fica_mais_conservador() {
        assert!(DECODE_FALLBACKS[0].tiling);
        assert_eq!(DECODE_FALLBACKS[1].vae, "f32");
        assert!(DECODE_FALLBACKS[2].cpu);
        let args = decode_args(
            Path::new("modelo"),
            Path::new("out.png"),
            "Vulkan0",
            DECODE_FALLBACKS[2],
        );
        assert!(args.iter().any(|arg| arg == "vae=cpu,Vulkan0"));
        assert!(args.iter().any(|arg| arg.contains("f32")));
    }

    fn flag_is(args: &[String], flag: &str, value: &str) -> bool {
        args.windows(2)
            .any(|pair| pair[0] == flag && pair[1] == value)
    }

    #[test]
    fn a_edicao_entra_nas_flags() {
        let vary = img2img_args(
            Path::new("modelo"),
            Path::new("out.png"),
            Path::new("prev.png"),
            "Vulkan0",
            &Img2ImgRequest {
                job_id: "job".to_owned(),
                prompt: "um gato".to_owned(),
                negative_prompt: None,
                model_id: "sd15".to_owned(),
                image_path: "origem.png".to_owned(),
                output_path: "out.png".to_owned(),
                steps: 20,
                strength: 0.4,
                width: Some(512),
                height: Some(512),
                seed: Some(7),
                cfg_scale: Some(7.0),
                circular: None,
            },
        );
        assert!(flag_is(&vary, "--init-img", "origem.png"));
        assert!(flag_is(&vary, "--strength", "0.40"));
        assert!(!vary.iter().any(|arg| arg == "--circular"));

        let paint = inpaint_args(
            Path::new("modelo"),
            Path::new("out.png"),
            Path::new("prev.png"),
            "Vulkan0",
            &InpaintRequest {
                job_id: "job".to_owned(),
                prompt: "um gato".to_owned(),
                negative_prompt: None,
                model_id: "sd15".to_owned(),
                image_path: "origem.png".to_owned(),
                mask_path: "mascara.png".to_owned(),
                output_path: "out.png".to_owned(),
                steps: 20,
                strength: 0.75,
                width: Some(512),
                height: Some(512),
                seed: Some(7),
                cfg_scale: None,
                circular: None,
            },
        );
        assert!(flag_is(&paint, "--mask", "mascara.png"));

        let tile = image_args(
            Path::new("modelo"),
            Path::new("out.png"),
            Path::new("prev.png"),
            "Vulkan0",
            &ImageJob {
                prompt: "tecido".to_owned(),
                negative_prompt: None,
                steps: 20,
                width: 512,
                height: 512,
                seed: 1,
                cfg_scale: 7.0,
                init_image: None,
                strength: None,
                mask: None,
                circular: true,
            },
        );
        assert!(tile.iter().any(|arg| arg == "--circular"));
    }
}
