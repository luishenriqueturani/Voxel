//! Tipos persistidos da fila.

use serde::{Deserialize, Serialize};

/// Parâmetros já resolvidos no momento de enfileirar.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueResolved {
    pub prompt: String,
    pub negative: String,
    #[serde(rename = "model2d")]
    pub model_2d: String,
    #[serde(rename = "model3d")]
    pub model_3d: String,
    pub resolution: i64,
    pub aspect: String,
    pub steps: i64,
    pub cfg: f64,
    pub seed: String,
    pub postprocess: bool,
    pub count: i64,
    pub mesh: String,
}

/// Item da fila, com o progresso e o log se falhou.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueItem {
    pub id: String,
    pub status: String,
    pub prompt: String,
    pub count: i64,
    pub mesh: String,
    pub images: i64,
    pub meshes: i64,
    pub resolved: QueueResolved,
    pub error_log: Option<String>,
    pub error_summary: Option<String>,
    pub image_paths: Vec<String>,
}

/// Job ativo ou já encerrado.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueJob {
    pub id: String,
    pub name: String,
    pub status: String,
    pub end_retry: bool,
    pub items: Vec<QueueItem>,
}

/// Arquivo gerado por um passo.
#[derive(Debug, Clone, PartialEq)]
pub struct QueueOutput {
    pub path: String,
    pub seed: Option<i64>,
}
