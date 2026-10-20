use std::path::{Path, PathBuf};

use voxel_protocol::{ImageBackend, Img2ImgRequest, InpaintRequest, Txt2ImgRequest, WorkerProcess};

/// Pedido já limitado, pronto para o worker de imagem.
pub struct ImageCall {
    pub mode: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub model_id: String,
    pub width: i64,
    pub height: i64,
    pub steps: i64,
    pub cfg: f64,
    pub seed: i64,
    pub strength: f64,
    pub source_path: Option<String>,
    pub mask_png: Option<String>,
    pub output: PathBuf,
    pub job_id: String,
}

#[derive(Debug)]
enum Prepared {
    Text(Txt2ImgRequest),
    Vary(Img2ImgRequest),
    Paint(InpaintRequest),
}

/// Envia txt2img, img2img ou inpaint conforme o modo da interface.
///
/// # Errors
///
/// Falha se faltar a imagem de origem, a máscara ou se o worker recusar o pedido.
pub fn submit(worker: &WorkerProcess, call: &ImageCall) -> Result<(), String> {
    let prepared = prepare(call)?;
    let result = match prepared {
        Prepared::Text(request) => worker.txt2img(&request),
        Prepared::Vary(request) => worker.img2img(&request),
        Prepared::Paint(request) => worker.inpaint(&request),
    };
    result.map(|_| ()).map_err(|error| error.to_string())
}

fn prepare(call: &ImageCall) -> Result<Prepared, String> {
    let output = call.output.display().to_string();
    let circular = (call.mode == "tile").then_some(true);
    match call.mode.as_str() {
        "vary" => Ok(Prepared::Vary(vary_request(call, &output)?)),
        "inpaint" => Ok(Prepared::Paint(paint_request(call, &output)?)),
        _ => Ok(Prepared::Text(Txt2ImgRequest {
            job_id: call.job_id.clone(),
            prompt: call.prompt.clone(),
            negative_prompt: call.negative_prompt.clone(),
            model_id: call.model_id.clone(),
            output_path: output,
            steps: call.steps,
            width: call.width,
            height: call.height,
            seed: call.seed,
            cfg_scale: Some(call.cfg),
            circular,
        })),
    }
}

fn vary_request(call: &ImageCall, output: &str) -> Result<Img2ImgRequest, String> {
    Ok(Img2ImgRequest {
        job_id: call.job_id.clone(),
        prompt: call.prompt.clone(),
        negative_prompt: call.negative_prompt.clone(),
        model_id: call.model_id.clone(),
        image_path: source_file(call, "abra uma imagem para variar")?,
        output_path: output.to_owned(),
        steps: call.steps,
        strength: strength_of(call.strength),
        width: Some(call.width),
        height: Some(call.height),
        seed: Some(call.seed),
        cfg_scale: Some(call.cfg),
        circular: None,
    })
}

fn paint_request(call: &ImageCall, output: &str) -> Result<InpaintRequest, String> {
    let image_path = source_file(call, "abra uma imagem para pintar")?;
    let mask = write_mask(&call.output, call.mask_png.as_deref())?;
    Ok(InpaintRequest {
        job_id: call.job_id.clone(),
        prompt: call.prompt.clone(),
        negative_prompt: call.negative_prompt.clone(),
        model_id: call.model_id.clone(),
        image_path,
        mask_path: mask.display().to_string(),
        output_path: output.to_owned(),
        steps: call.steps,
        strength: strength_of(call.strength),
        width: Some(call.width),
        height: Some(call.height),
        seed: Some(call.seed),
        cfg_scale: Some(call.cfg),
        circular: None,
    })
}

fn source_file(call: &ImageCall, missing: &str) -> Result<String, String> {
    let Some(path) = call.source_path.as_deref() else {
        return Err(missing.to_owned());
    };
    if Path::new(path).is_file() {
        Ok(path.to_owned())
    } else {
        Err(missing.to_owned())
    }
}

fn strength_of(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.05, 1.0)
    } else {
        0.75
    }
}

fn write_mask(output: &Path, encoded: Option<&str>) -> Result<PathBuf, String> {
    let Some(encoded) = encoded.filter(|value| !value.is_empty()) else {
        return Err("pinte a área que deve mudar".to_owned());
    };
    let bytes = decode_png(encoded)?;
    let path = output.with_file_name("mascara.png");
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    std::fs::write(&path, bytes).map_err(|error| error.to_string())?;
    Ok(path)
}

fn decode_png(encoded: &str) -> Result<Vec<u8>, String> {
    let data = encoded
        .split_once(',')
        .map_or(encoded, |(_, payload)| payload);
    let bytes = decode_base64(data)?;
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok(bytes)
    } else {
        Err("máscara inválida".to_owned())
    }
}

fn decode_base64(text: &str) -> Result<Vec<u8>, String> {
    let compact: String = text
        .chars()
        .filter(|symbol| !symbol.is_whitespace())
        .collect();
    let bytes = compact.as_bytes();
    if bytes.is_empty() || !bytes.len().is_multiple_of(4) {
        return Err("máscara inválida".to_owned());
    }
    let mut out = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        let (chunk, pads) = chunk_at(bytes, index)?;
        push_byte(&mut out, chunk >> 16)?;
        if pads < 2 {
            push_byte(&mut out, chunk >> 8)?;
        }
        if pads < 1 {
            push_byte(&mut out, chunk)?;
        }
        index += 4;
    }
    Ok(out)
}

fn chunk_at(bytes: &[u8], index: usize) -> Result<(u32, usize), String> {
    let third_pad = bytes[index + 2] == b'=';
    let fourth_pad = bytes[index + 3] == b'=';
    if third_pad && !fourth_pad {
        return Err("máscara inválida".to_owned());
    }
    let first = sextet(bytes[index])?;
    let second = sextet(bytes[index + 1])?;
    let third = if third_pad {
        0
    } else {
        sextet(bytes[index + 2])?
    };
    let fourth = if fourth_pad {
        0
    } else {
        sextet(bytes[index + 3])?
    };
    let pads = usize::from(third_pad) + usize::from(fourth_pad);
    let chunk = (u32::from(first) << 18)
        | (u32::from(second) << 12)
        | (u32::from(third) << 6)
        | u32::from(fourth);
    Ok((chunk, pads))
}

fn sextet(byte: u8) -> Result<u8, String> {
    match byte {
        b'A'..=b'Z' => Ok(byte - b'A'),
        b'a'..=b'z' => Ok(byte - b'a' + 26),
        b'0'..=b'9' => Ok(byte - b'0' + 52),
        b'+' => Ok(62),
        b'/' => Ok(63),
        _ => Err("máscara inválida".to_owned()),
    }
}

fn push_byte(out: &mut Vec<u8>, value: u32) -> Result<(), String> {
    let byte = u8::try_from(value & 0xff).map_err(|_| "máscara inválida".to_owned())?;
    out.push(byte);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::encode_base64;

    fn call(mode: &str, folder: &Path) -> ImageCall {
        ImageCall {
            mode: mode.to_owned(),
            prompt: "um gato".to_owned(),
            negative_prompt: None,
            model_id: "sd15".to_owned(),
            width: 512,
            height: 512,
            steps: 20,
            cfg: 7.0,
            seed: 4,
            strength: 0.4,
            source_path: None,
            mask_png: None,
            output: folder.join("imagem_01.png"),
            job_id: "job".to_owned(),
        }
    }

    #[test]
    fn a_textura_pede_padding_circular_e_a_nova_nao() {
        let folder = std::env::temp_dir().join(format!("voxel-edit-{}", std::process::id()));
        let tile = prepare(&call("tile", &folder)).unwrap();
        let Prepared::Text(request) = tile else {
            panic!("textura");
        };
        assert_eq!(request.circular, Some(true));
        let fresh = prepare(&call("new", &folder)).unwrap();
        let Prepared::Text(request) = fresh else {
            panic!("nova");
        };
        assert_eq!(request.circular, None);
    }

    #[test]
    fn variar_sem_arquivo_falha_e_com_arquivo_guarda_a_intensidade() {
        let folder = std::env::temp_dir().join(format!("voxel-vary-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let missing = prepare(&call("vary", &folder)).unwrap_err();
        assert_eq!(missing, "abra uma imagem para variar");

        let source = folder.join("origem.png");
        std::fs::write(&source, "png").unwrap();
        let mut ready = call("vary", &folder);
        ready.source_path = Some(source.display().to_string());
        ready.strength = 4.0;
        let Prepared::Vary(request) = prepare(&ready).unwrap() else {
            panic!("variar");
        };
        assert!((request.strength - 1.0).abs() < f64::EPSILON);
        assert_eq!(request.image_path, source.display().to_string());
        let _ = std::fs::remove_dir_all(&folder);
    }

    #[test]
    fn a_mascara_sai_em_png_ao_lado_da_imagem() {
        let folder = std::env::temp_dir().join(format!("voxel-mask-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let source = folder.join("origem.png");
        std::fs::write(&source, "png").unwrap();
        let magic = b"\x89PNG\r\n\x1a\nextra";
        let mut ready = call("inpaint", &folder);
        ready.source_path = Some(source.display().to_string());
        ready.mask_png = Some(format!("data:image/png;base64,{}", encode_base64(magic)));
        let Prepared::Paint(request) = prepare(&ready).unwrap() else {
            panic!("pintar");
        };
        let written = std::fs::read(folder.join("mascara.png")).unwrap();
        assert_eq!(written, magic);
        assert_eq!(
            request.mask_path,
            folder.join("mascara.png").display().to_string()
        );
        ready.mask_png = None;
        assert_eq!(prepare(&ready).unwrap_err(), "pinte a área que deve mudar");
        let _ = std::fs::remove_dir_all(&folder);
    }
}
