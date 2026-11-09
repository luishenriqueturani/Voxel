//! Fila persistida: jobs, itens, saídas e logs de falha isolada.

mod control;
mod picks;
mod report;
mod store;
mod types;

use std::sync::atomic::{AtomicU64, Ordering};

pub use types::{MeshPick, QueueItem, QueueJob, QueueOutput, QueueResolved, QueueThumb};
use voxel_queue::{ItemProgress, ItemStatus};

use super::flows::{FlowItem, MeshMode as FlowMesh};
use super::{Database, PromptTemplate};
use crate::output::unix_now;

impl Database {
    /// Progresso no formato da máquina de estados.
    #[must_use]
    pub fn queue_progress(job: &QueueJob) -> Vec<ItemProgress> {
        job.items
            .iter()
            .map(|item| ItemProgress {
                status: parse_status(&item.status),
                mesh: parse_mesh(&item.mesh),
                count: u32::try_from(item.count).unwrap_or(1).max(1),
                images: u32::try_from(item.images).unwrap_or(0),
                meshes: u32::try_from(item.meshes).unwrap_or(0),
            })
            .collect()
    }
}

fn parse_status(value: &str) -> ItemStatus {
    match value {
        "images" => ItemStatus::Images,
        "mesh" => ItemStatus::Mesh,
        "done" => ItemStatus::Done,
        "failed" => ItemStatus::Failed,
        "cancelled" => ItemStatus::Cancelled,
        _ => ItemStatus::Pending,
    }
}

fn status_value(status: ItemStatus) -> &'static str {
    match status {
        ItemStatus::Pending => "pending",
        ItemStatus::Images => "images",
        ItemStatus::Mesh => "mesh",
        ItemStatus::Done => "done",
        ItemStatus::Failed => "failed",
        ItemStatus::Cancelled => "cancelled",
    }
}

fn parse_mesh(value: &str) -> voxel_queue::MeshMode {
    match value {
        "all" => voxel_queue::MeshMode::All,
        "later" => voxel_queue::MeshMode::Later,
        _ => voxel_queue::MeshMode::None,
    }
}

fn mesh_value(mode: FlowMesh) -> &'static str {
    match mode {
        FlowMesh::None => "none",
        FlowMesh::All => "all",
        FlowMesh::Later => "later",
    }
}

fn resolve_item(item: &FlowItem, template: Option<&PromptTemplate>) -> QueueResolved {
    let item_negative = item.overrides.negative.clone().unwrap_or_default();
    let (prompt, negative) = match template {
        Some(template) => template.compose(&item.prompt, &item_negative),
        None => (item.prompt.clone(), item_negative),
    };
    QueueResolved {
        prompt,
        negative,
        model_2d: item
            .overrides
            .model_2d
            .clone()
            .unwrap_or_else(|| "sd15".to_owned()),
        model_3d: item
            .overrides
            .model_3d
            .clone()
            .unwrap_or_else(|| "triposr".to_owned()),
        resolution: item.overrides.resolution.unwrap_or(512),
        aspect: item
            .overrides
            .aspect
            .clone()
            .unwrap_or_else(|| "1:1".to_owned()),
        steps: item.overrides.steps.unwrap_or(20),
        cfg: item.overrides.cfg.unwrap_or(7.0),
        seed: item.overrides.seed.clone().unwrap_or_default(),
        postprocess: item.overrides.postprocess.unwrap_or(false),
        count: item.count,
        mesh: mesh_value(item.mesh).to_owned(),
    }
}

fn new_id() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
    format!("{}-q{sequence}", unix_now())
}
