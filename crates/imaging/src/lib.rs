//! Remoção de fundo e utilidades de processamento de imagem.
//!
//! O U²-Net roda em CPU, pela crate `ort`. O pré-processamento segue o rembg:
//! 320×320, divisão pelo máximo do RGB e normalização `ImageNet`.

mod error;
mod session;
mod tensor;
mod thumb;

use std::path::{Path, PathBuf};

use voxel_protocol::{BackgroundRemover, JobOutput, RemoveBackgroundRequest, WorkerError};

pub use error::ImagingError;
pub use thumb::thumbnail_jpeg;

/// PNG com o alfa já resolvido, pronto para a prévia.
pub struct PngImage {
    pub bytes: Vec<u8>,
    pub transparent: bool,
}

/// U²-Net em CPU, via ONNX Runtime.
pub struct OnnxBackgroundRemover {
    model: PathBuf,
}

impl OnnxBackgroundRemover {
    /// Guarda o caminho do `u2net.onnx`. A sessão abre na hora de remover o fundo.
    #[must_use]
    pub fn new(model: PathBuf) -> Self {
        Self { model }
    }
}

impl BackgroundRemover for OnnxBackgroundRemover {
    fn remove_background(
        &self,
        request: &RemoveBackgroundRequest,
    ) -> Result<JobOutput, WorkerError> {
        remove_background(
            &self.model,
            Path::new(&request.image_path),
            Path::new(&request.output_path),
        )
        .map_err(|error| WorkerError::Protocol(error.to_string()))?;
        Ok(JobOutput {
            job_id: request.job_id.clone(),
            path: request.output_path.clone(),
        })
    }
}

/// Remove o fundo e grava um PNG com alfa.
///
/// # Errors
///
/// Falha se o modelo ou a imagem não puderem ser lidos, ou se a máscara vier em formato inesperado.
pub fn remove_background(model: &Path, source: &Path, output: &Path) -> Result<(), ImagingError> {
    session::cut(model, source, output)
}

/// Lê a imagem e devolve um PNG. `transparent` é verdadeiro se algum pixel já tem alfa.
///
/// # Errors
///
/// Falha se o arquivo não for uma imagem suportada.
pub fn read_png(path: &Path) -> Result<PngImage, ImagingError> {
    let image = image::open(path)?.into_rgba8();
    let transparent = tensor::has_transparency(&image);
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgba8(image).write_to(
        &mut std::io::Cursor::new(&mut bytes),
        image::ImageFormat::Png,
    )?;
    Ok(PngImage { bytes, transparent })
}

/// Caminho irmão `{stem}_sem_fundo.png`. Se já existir, numera a partir de 2.
#[must_use]
pub fn sibling_cutout(source: &Path) -> PathBuf {
    let parent = source.parent().unwrap_or(Path::new("."));
    let stem = source
        .file_stem()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("imagem");
    for index in 1..=999 {
        let name = if index == 1 {
            format!("{stem}_sem_fundo.png")
        } else {
            format!("{stem}_sem_fundo_{index}.png")
        };
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem}_sem_fundo_999.png"))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use image::{Rgba, RgbaImage};
    use voxel_protocol::{BackgroundRemover, RemoveBackgroundRequest};

    use super::{OnnxBackgroundRemover, read_png, remove_background, sibling_cutout};

    #[test]
    fn png_transparente_e_o_irmao_numerado() {
        let dir = std::env::temp_dir().join(format!("voxel-cutout-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("pasta");
        let source = dir.join("imagem_01.png");
        RgbaImage::from_pixel(1, 1, Rgba([1, 2, 3, 0]))
            .save(&source)
            .expect("png");
        let loaded = read_png(&source).expect("leitura");
        assert!(loaded.transparent);
        assert!(loaded.bytes.starts_with(b"\x89PNG"));

        let first = sibling_cutout(&source);
        assert!(first.ends_with("imagem_01_sem_fundo.png"));
        std::fs::write(&first, b"x").expect("ocupa");
        let second = sibling_cutout(&source);
        assert!(second.ends_with("imagem_01_sem_fundo_2.png"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn removedor_recusa_modelo_ausente() {
        let remover = OnnxBackgroundRemover::new(PathBuf::from("/nao/existe/u2net.onnx"));
        let error = remover
            .remove_background(&RemoveBackgroundRequest {
                job_id: "job".to_owned(),
                image_path: "/nao/existe.png".to_owned(),
                output_path: "/tmp/saida.png".to_owned(),
            })
            .expect_err("modelo");
        assert!(error.to_string().contains("u2net"));
    }

    #[test]
    fn u2net_separa_um_circulo_quando_o_modelo_existe() {
        let Some(model) = std::env::var_os("VOXEL_U2NET") else {
            return;
        };
        let model = PathBuf::from(model);
        if !model.is_file() {
            return;
        }
        let mut image = RgbaImage::new(128, 128);
        for y in 0..128 {
            for x in 0..128 {
                let dx = f32::from(u16::try_from(x).unwrap_or(0)) - 64.0;
                let dy = f32::from(u16::try_from(y).unwrap_or(0)) - 64.0;
                let inside = dx.mul_add(dx, dy * dy) < 30.0 * 30.0;
                let color = if inside { 240 } else { 20 };
                image.put_pixel(x, y, Rgba([color, color, color, 255]));
            }
        }
        let dir = std::env::temp_dir().join(format!("voxel-u2net-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("pasta");
        let source = dir.join("circulo.png");
        let output = dir.join("circulo_sem_fundo.png");
        image.save(&source).expect("origem");
        remove_background(&model, &source, &output).expect("recorte");
        remove_background(&model, &source, &output).expect("segundo recorte");
        let cut = image::open(&output).expect("saída").into_rgba8();
        let center = cut.get_pixel(64, 64)[3];
        let corner = cut.get_pixel(0, 0)[3];
        assert!(center > 200, "centro {center}");
        assert!(corner < 80, "canto {corner}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
