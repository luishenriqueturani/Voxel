// Gerado a partir de protocol/openapi.json. Não edite à mão.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Health {
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Ack {
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ErrorBody {
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelKind {
    #[serde(rename = "image")]
    Image,
    #[serde(rename = "mesh")]
    Mesh,
    #[serde(rename = "background")]
    Background,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct LoadModelRequest {
    pub model_id: String,
    pub model_path: String,
    pub kind: ModelKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct CancelRequest {
    pub job_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Txt2ImgRequest {
    pub job_id: String,
    pub prompt: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub negative_prompt: Option<String>,
    pub model_id: String,
    pub output_path: String,
    pub steps: i64,
    pub width: i64,
    pub height: i64,
    pub seed: i64,
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct Img2ImgRequest {
    pub job_id: String,
    pub prompt: String,
    pub model_id: String,
    pub image_path: String,
    pub output_path: String,
    pub steps: i64,
    pub strength: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct InpaintRequest {
    pub job_id: String,
    pub prompt: String,
    pub model_id: String,
    pub image_path: String,
    pub mask_path: String,
    pub output_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct DecodeLatentsRequest {
    pub job_id: String,
    pub model_id: String,
    pub latents_path: String,
    pub output_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct RemoveBackgroundRequest {
    pub job_id: String,
    pub image_path: String,
    pub output_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ImageToMeshRequest {
    pub job_id: String,
    pub image_path: String,
    pub model_id: String,
    pub output_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct MeshPostProcessRequest {
    pub job_id: String,
    pub mesh_path: String,
    pub output_path: String,
    pub decimate: bool,
    pub normalize: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_textures: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct JobOutput {
    pub job_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProgressName {
    #[serde(rename = "progress")]
    Progress,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletedName {
    #[serde(rename = "completed")]
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailedName {
    #[serde(rename = "failed")]
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct ProgressEvent {
    pub event: ProgressName,
    pub job_id: String,
    pub step: i64,
    pub total: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct CompletedEvent {
    pub event: CompletedName,
    pub job_id: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(deny_unknown_fields)]
pub struct FailedEvent {
    pub event: FailedName,
    pub job_id: String,
    pub message: String,
}

pub mod paths {
    pub const HEALTH: &str = "/health";
    pub const MODELS_LOAD: &str = "/models/load";
    pub const MODELS_UNLOAD: &str = "/models/unload";
    pub const JOBS_CANCEL: &str = "/jobs/cancel";
    pub const IMAGES_TXT2IMG: &str = "/images/txt2img";
    pub const IMAGES_IMG2IMG: &str = "/images/img2img";
    pub const IMAGES_INPAINT: &str = "/images/inpaint";
    pub const IMAGES_DECODE_LATENTS: &str = "/images/decode-latents";
    pub const IMAGES_REMOVE_BACKGROUND: &str = "/images/remove-background";
    pub const MESHES_FROM_IMAGE: &str = "/meshes/from-image";
    pub const MESHES_POST_PROCESS: &str = "/meshes/post-process";
    pub const EVENTS: &str = "/events";
}
