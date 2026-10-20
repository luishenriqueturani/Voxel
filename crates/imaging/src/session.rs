use std::path::Path;
use std::sync::Mutex;

use ort::environment::Environment;
use ort::session::Session;
use ort::value::{Shape, Tensor};

use crate::error::ImagingError;
use crate::tensor::{self, INPUT};

pub(crate) fn cut(model: &Path, source: &Path, output: &Path) -> Result<(), ImagingError> {
    if !model.is_file() {
        return Err(ImagingError::Model("modelo u2net ausente".to_owned()));
    }
    let rgba = image::open(source)?.into_rgba8();
    if rgba.width() == 0 || rgba.height() == 0 {
        return Err(ImagingError::Shape("imagem vazia".to_owned()));
    }
    let rgb = image::DynamicImage::ImageRgba8(rgba.clone()).into_rgb8();
    let plane = infer(model, tensor::input_tensor(&rgb))?;
    let mask = tensor::mask_bytes(
        &plane.values,
        plane.width,
        plane.height,
        rgba.width(),
        rgba.height(),
    )?;
    let cut = tensor::apply_alpha(&rgba, &mask)?;
    cut.save(output)?;
    Ok(())
}

struct Plane {
    width: u32,
    height: u32,
    values: Vec<f32>,
}

fn infer(model: &Path, input: Vec<f32>) -> Result<Plane, ImagingError> {
    let environment = shared_environment()?;
    let mut session = Session::builder(&environment)
        .map_err(runtime)?
        .with_intra_threads(intra_threads())
        .map_err(runtime)?
        .commit_from_file(model)
        .map_err(runtime)?;
    let tensor = Tensor::<f32>::from_array((
        Shape::new([1_i64, 3, i64::from(INPUT), i64::from(INPUT)]),
        input,
    ))
    .map_err(runtime)?;
    let outputs = session.run(ort::inputs![tensor]).map_err(runtime)?;
    let (shape, values) = outputs[0].try_extract_tensor::<f32>().map_err(runtime)?;
    let (width, height) = plane_size(shape)?;
    let count = usize::try_from(width)?
        .checked_mul(usize::try_from(height)?)
        .ok_or_else(|| ImagingError::Shape("máscara grande demais".to_owned()))?;
    if values.len() < count {
        return Err(ImagingError::Shape("saída curta".to_owned()));
    }
    Ok(Plane {
        width,
        height,
        values: values[..count].to_vec(),
    })
}

fn plane_size(shape: &[i64]) -> Result<(u32, u32), ImagingError> {
    let (height, width) = match shape {
        [height, width] | [_, height, width] | [_, _, height, width] => (*height, *width),
        _ => {
            return Err(ImagingError::Shape(format!("formato {shape:?}")));
        }
    };
    if height <= 0 || width <= 0 {
        return Err(ImagingError::Shape("máscara vazia".to_owned()));
    }
    Ok((u32::try_from(width)?, u32::try_from(height)?))
}

/// O ONNX Runtime recusa um segundo ambiente no mesmo processo, mesmo depois que o primeiro caiu.
fn shared_environment() -> Result<Environment, ImagingError> {
    static ENV: Mutex<Option<Environment>> = Mutex::new(None);
    let mut slot = ENV
        .lock()
        .map_err(|_| ImagingError::Runtime("ambiente onnx ocupado".to_owned()))?;
    if let Some(environment) = slot.as_ref() {
        return Ok(environment.clone());
    }
    let environment = ort::init()
        .with_name("voxel")
        .with_telemetry(false)
        .build()
        .map_err(runtime)?;
    *slot = Some(environment.clone());
    Ok(environment)
}

fn intra_threads() -> usize {
    std::thread::available_parallelism().map_or(1, |count| count.get().clamp(1, 8))
}

fn runtime(error: impl std::fmt::Display) -> ImagingError {
    ImagingError::Runtime(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::shared_environment;

    #[test]
    fn o_ambiente_onnx_serve_mais_de_uma_vez() {
        let first = shared_environment().expect("abrir");
        drop(first);
        shared_environment().expect("reusar");
    }
}
