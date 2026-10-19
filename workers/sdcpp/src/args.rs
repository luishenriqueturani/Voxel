use std::path::Path;

use voxel_protocol::Txt2ImgRequest;

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
    let mut args = vec![
        "-m".to_owned(),
        model.display().to_string(),
        "-p".to_owned(),
        request.prompt.clone(),
        "-o".to_owned(),
        output.display().to_string(),
        "--preview".to_owned(),
        "proj".to_owned(),
        "--preview-path".to_owned(),
        preview.display().to_string(),
        "--preview-interval".to_owned(),
        "1".to_owned(),
        "--steps".to_owned(),
        request.steps.to_string(),
        "-W".to_owned(),
        request.width.to_string(),
        "-H".to_owned(),
        request.height.to_string(),
        "--seed".to_owned(),
        request.seed.to_string(),
        "--sampling-method".to_owned(),
        "euler_a".to_owned(),
        "--backend".to_owned(),
        device.to_owned(),
        "--tensor-type-rules".to_owned(),
        r"^vae\.=f16".to_owned(),
        "--auto-fit".to_owned(),
        "on".to_owned(),
    ];
    if let Some(negative) = &request.negative_prompt {
        args.push("-n".to_owned());
        args.push(negative.clone());
    }
    args
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
}
