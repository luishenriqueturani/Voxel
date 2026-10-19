//! Tipos do protocolo de comunicação entre o core e os workers de inferência.
//!
//! O esquema em `protocol/openapi.json` é a fonte. Os tipos de Rust, TypeScript e Python
//! saem dele, para as três linguagens não divergirem.

mod codegen;
mod error;
mod generated;
mod supervisor;
mod traits;

pub use codegen::{Generated, generate};
pub use error::{SchemaError, WorkerError};
#[allow(clippy::wildcard_imports)]
pub use generated::*;
pub use supervisor::{WorkerEvent, WorkerProcess};
pub use traits::{BackgroundRemover, ImageBackend, MeshBackend, MeshPostProcess};
pub use voxel_logging::ProcessOutcome;
