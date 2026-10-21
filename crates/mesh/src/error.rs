use std::fmt;

/// Falha ao ler, reduzir ou gravar a malha.
#[derive(Debug)]
pub enum MeshError {
    Read(String),
    Write(String),
    Empty,
    Range(String),
}

impl fmt::Display for MeshError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Write(detail) => write!(formatter, "gravação: {detail}"),
            Self::Empty => write!(formatter, "malha vazia"),
            Self::Read(detail) | Self::Range(detail) => write!(formatter, "malha: {detail}"),
        }
    }
}

impl std::error::Error for MeshError {}

impl From<std::io::Error> for MeshError {
    fn from(error: std::io::Error) -> Self {
        Self::Write(error.to_string())
    }
}

impl From<gltf::Error> for MeshError {
    fn from(error: gltf::Error) -> Self {
        Self::Read(error.to_string())
    }
}

impl From<image::ImageError> for MeshError {
    fn from(error: image::ImageError) -> Self {
        Self::Write(error.to_string())
    }
}

impl From<std::num::TryFromIntError> for MeshError {
    fn from(error: std::num::TryFromIntError) -> Self {
        Self::Range(error.to_string())
    }
}
