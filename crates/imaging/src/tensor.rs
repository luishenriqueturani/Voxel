use image::imageops::FilterType;
use image::{GrayImage, RgbImage, RgbaImage};

use crate::error::ImagingError;

pub(crate) const INPUT: u32 = 320;

const MEAN: [f32; 3] = [0.485, 0.456, 0.406];
const STD: [f32; 3] = [0.229, 0.224, 0.225];

/// RGB em 320×320, normalizado como o rembg faz com o U²-Net: divide pelo máximo e aplica `ImageNet`.
pub(crate) fn input_tensor(image: &RgbImage) -> Vec<f32> {
    let resized = if image.width() == INPUT && image.height() == INPUT {
        image.clone()
    } else {
        image::imageops::resize(image, INPUT, INPUT, FilterType::Lanczos3)
    };
    let max = resized.pixels().flat_map(|pixel| pixel.0).fold(0, u8::max);
    let scale = f32::from(max).max(1e-6);
    let side = usize::try_from(INPUT).unwrap_or(0);
    let plane = side.saturating_mul(side);
    let mut data = vec![0.0; 3 * plane];
    for (index, pixel) in resized.pixels().enumerate() {
        for channel in 0..3 {
            let value = f32::from(pixel.0[channel]) / scale;
            data[channel * plane + index] = (value - MEAN[channel]) / STD[channel];
        }
    }
    data
}

/// Normaliza a saída do modelo para 0–255 e devolve ao tamanho da imagem.
///
/// # Errors
///
/// Falha se a saída for menor que o plano informado ou grande demais para a máscara.
pub(crate) fn mask_bytes(
    values: &[f32],
    from_width: u32,
    from_height: u32,
    to_width: u32,
    to_height: u32,
) -> Result<Vec<u8>, ImagingError> {
    let from_count = pixel_count(from_width, from_height)?;
    if values.len() < from_count {
        return Err(ImagingError::Shape("máscara curta".to_owned()));
    }
    let plane = &values[..from_count];
    let min = plane.iter().copied().fold(f32::INFINITY, f32::min);
    let max = plane.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    let span = (max - min).max(1e-6);
    let bytes: Vec<u8> = plane
        .iter()
        .map(|value| byte_from_unit((value - min) / span))
        .collect();
    if from_width == to_width && from_height == to_height {
        return Ok(bytes);
    }
    let image = GrayImage::from_raw(from_width, from_height, bytes)
        .ok_or_else(|| ImagingError::Shape("máscara inválida".to_owned()))?;
    let resized = image::imageops::resize(&image, to_width, to_height, FilterType::Lanczos3);
    Ok(resized.into_raw())
}

/// Troca o alfa pelo valor da máscara. O RGB original permanece.
///
/// # Errors
///
/// Falha se a máscara não tiver um byte por pixel.
pub(crate) fn apply_alpha(image: &RgbaImage, mask: &[u8]) -> Result<RgbaImage, ImagingError> {
    let count = pixel_count(image.width(), image.height())?;
    if mask.len() != count {
        return Err(ImagingError::Shape("alfa diferente da imagem".to_owned()));
    }
    let mut cut = image.clone();
    for (pixel, alpha) in cut.pixels_mut().zip(mask) {
        pixel[3] = *alpha;
    }
    Ok(cut)
}

#[must_use]
pub(crate) fn has_transparency(image: &RgbaImage) -> bool {
    image.pixels().any(|pixel| pixel[3] < 255)
}

fn pixel_count(width: u32, height: u32) -> Result<usize, ImagingError> {
    let width = usize::try_from(width)?;
    let height = usize::try_from(height)?;
    width
        .checked_mul(height)
        .ok_or_else(|| ImagingError::Shape("imagem grande demais".to_owned()))
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn byte_from_unit(unit: f32) -> u8 {
    let scaled = (unit.clamp(0.0, 1.0) * 255.0).round();
    if scaled >= 255.0 {
        255
    } else if scaled <= 0.0 {
        0
    } else {
        scaled as u8
    }
}

#[cfg(test)]
mod tests {
    use image::{Rgba, RgbaImage};

    use super::{INPUT, apply_alpha, has_transparency, input_tensor, mask_bytes};

    #[test]
    fn tensor_de_branco_segue_o_imagenet() {
        let image = image::RgbImage::from_pixel(INPUT, INPUT, image::Rgb([255, 255, 255]));
        let data = input_tensor(&image);
        let plane = usize::try_from(INPUT).expect("lado") * usize::try_from(INPUT).expect("lado");
        let red = (1.0 - 0.485) / 0.229;
        let green = (1.0 - 0.456) / 0.224;
        let blue = (1.0 - 0.406) / 0.225;
        assert!((data[0] - red).abs() < 1e-4);
        assert!((data[plane] - green).abs() < 1e-4);
        assert!((data[2 * plane] - blue).abs() < 1e-4);
    }

    #[test]
    fn mascara_normaliza_e_o_valor_constante_fica_vazio() {
        assert_eq!(
            mask_bytes(&[0.0, 1.0, 0.5, 0.25], 2, 2, 2, 2).expect("máscara"),
            vec![0, 255, 128, 64]
        );
        assert_eq!(
            mask_bytes(&[0.4, 0.4], 1, 2, 1, 2).expect("constante"),
            vec![0, 0]
        );
        assert_eq!(
            mask_bytes(&[0.0, 1.0], 2, 1, 4, 2).expect("resize").len(),
            8
        );
    }

    #[test]
    fn alfa_substitui_o_da_imagem_e_detecta_transparencia() {
        let image = RgbaImage::from_pixel(1, 1, Rgba([10, 20, 30, 255]));
        assert!(!has_transparency(&image));
        let cut = apply_alpha(&image, &[0]).expect("alfa");
        assert_eq!(cut.get_pixel(0, 0).0, [10, 20, 30, 0]);
        assert!(has_transparency(&cut));
        assert!(apply_alpha(&image, &[]).is_err());
    }
}
