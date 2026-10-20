//! Miniatura JPEG para a galeria. A imagem inteira não cabe no canal da janela.

use std::io::Cursor;
use std::path::Path;

use crate::ImagingError;

/// Reduz a imagem para caber em `edge`×`edge` e devolve um JPEG.
///
/// # Errors
///
/// Falha se o arquivo não for uma imagem ou se o JPEG não puder ser gravado.
pub fn thumbnail_jpeg(path: &Path, edge: u32) -> Result<Vec<u8>, ImagingError> {
    let image = image::open(path)?;
    let thumb = image.thumbnail(edge.max(1), edge.max(1)).into_rgb8();
    let mut bytes = Vec::new();
    image::DynamicImage::ImageRgb8(thumb)
        .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Jpeg)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::thumbnail_jpeg;

    #[test]
    fn miniatura_de_png_sai_em_jpeg() {
        let path = PathBuf::from(format!("/tmp/voxel-thumb-{}.png", std::process::id()));
        let image = image::RgbaImage::from_pixel(8, 4, image::Rgba([10, 20, 30, 255]));
        image.save(&path).unwrap();
        let jpeg = thumbnail_jpeg(&path, 2).unwrap();
        let _ = std::fs::remove_file(&path);
        assert!(jpeg.starts_with(&[0xff, 0xd8]));
    }

    #[test]
    fn arquivo_ausente_falha() {
        let missing = PathBuf::from(format!("/tmp/voxel-thumb-ausente-{}", std::process::id()));
        assert!(thumbnail_jpeg(&missing, 16).is_err());
    }
}
