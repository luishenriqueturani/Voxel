use std::fmt;

/// Falha ao ler a imagem, o modelo ou a máscara.
#[derive(Debug)]
pub enum ImagingError {
    Image(image::ImageError),
    Model(String),
    Shape(String),
    Io(std::io::Error),
    Runtime(String),
}

impl fmt::Display for ImagingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Image(error) => write!(formatter, "imagem: {error}"),
            Self::Model(detail) => write!(formatter, "{detail}"),
            Self::Shape(detail) => write!(formatter, "máscara: {detail}"),
            Self::Io(error) => write!(formatter, "io: {error}"),
            Self::Runtime(detail) => write!(formatter, "onnx: {detail}"),
        }
    }
}

impl std::error::Error for ImagingError {}

impl From<image::ImageError> for ImagingError {
    fn from(error: image::ImageError) -> Self {
        Self::Image(error)
    }
}

impl From<std::io::Error> for ImagingError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<std::num::TryFromIntError> for ImagingError {
    fn from(error: std::num::TryFromIntError) -> Self {
        Self::Shape(error.to_string())
    }
}
