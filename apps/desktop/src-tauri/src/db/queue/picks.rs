//! Segunda fila só de 3D, a partir das imagens escolhidas.

use rusqlite::params;

use super::new_id;
use super::report::waiting_later;
use super::types::{MeshPick, QueueJob};
use crate::db::{Database, DbError};
use crate::output::{unix_now, utc_timestamp};

impl Database {
    /// Cria um job que gera só a malha das imagens escolhidas.
    ///
    /// # Errors
    ///
    /// Falha se o job não existir, se as imagens não forem do item ou se nada foi escolhido.
    pub fn enqueue_mesh_picks(
        &self,
        job_id: &str,
        picks: &[MeshPick],
    ) -> Result<QueueJob, DbError> {
        let source = self.queue_job(job_id)?;
        let chosen = chosen_picks(&source, picks)?;
        let flow_id: Option<String> = {
            let connection = self.lock()?;
            connection
                .query_row(
                    "SELECT flow_id FROM queue_jobs WHERE id = ?1",
                    [job_id],
                    |row| row.get(0),
                )
                .map_err(|error| match error {
                    rusqlite::Error::QueryReturnedNoRows => DbError::Missing,
                    other => DbError::from(other),
                })?
        };
        let mesh_id = new_id();
        let now = utc_timestamp(unix_now());
        let name = format!("{} · 3D", source.name);
        {
            let connection = self.lock()?;
            connection.execute(
                "INSERT INTO queue_jobs (id, flow_id, name, status, end_retry, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'pending', 0, ?4, ?4)",
                params![mesh_id, flow_id, name, now],
            )?;
            for (position, (item, paths)) in chosen.iter().enumerate() {
                let mut resolved = item.resolved.clone();
                resolved.mesh.clear();
                resolved.mesh.push_str("all");
                resolved.count = i64::try_from(paths.len()).unwrap_or(1).max(1);
                let payload = serde_json::to_string(&resolved)
                    .map_err(|_| DbError::Invalid("item de fila inválido"))?;
                let item_id = new_id();
                let count = resolved.count;
                let index =
                    i64::try_from(position).map_err(|_| DbError::Invalid("posição inválida"))?;
                connection.execute(
                    "INSERT INTO queue_items (
                        id, job_id, position, flow_item_id, status, mesh, count, images, meshes,
                        resolved, error_log, error_summary
                     ) VALUES (?1, ?2, ?3, ?4, 'mesh', 'all', ?5, ?5, 0, ?6, NULL, NULL)",
                    params![item_id, mesh_id, index, item.id, count, payload],
                )?;
                for (offset, path) in paths.iter().enumerate() {
                    let place =
                        i64::try_from(offset).map_err(|_| DbError::Invalid("posição inválida"))?;
                    connection.execute(
                        "INSERT INTO queue_outputs (id, item_id, kind, path, seed, position)
                         VALUES (?1, ?2, 'image', ?3, NULL, ?4)",
                        params![new_id(), item_id, path, place],
                    )?;
                }
            }
        }
        self.queue_job(&mesh_id)
    }
}

fn chosen_picks<'a>(
    source: &'a QueueJob,
    picks: &'a [MeshPick],
) -> Result<Vec<(&'a crate::db::QueueItem, &'a [String])>, DbError> {
    let mut chosen = Vec::new();
    for pick in picks {
        if pick.paths.is_empty() {
            continue;
        }
        let item = source
            .items
            .iter()
            .find(|item| item.id == pick.item_id)
            .ok_or(DbError::Invalid("item fora do job"))?;
        if !waiting_later(item) {
            return Err(DbError::Invalid("item não espera escolha"));
        }
        if pick
            .paths
            .iter()
            .any(|path| !item.image_paths.iter().any(|known| known == path))
        {
            return Err(DbError::Invalid("imagem fora do item"));
        }
        chosen.push((item, pick.paths.as_slice()));
    }
    if chosen.is_empty() {
        return Err(DbError::Invalid("nenhuma imagem escolhida"));
    }
    Ok(chosen)
}

#[cfg(test)]
mod tests {
    use super::super::report::waiting_later;
    use crate::db::flows::{FlowDraft, FlowItem, FlowOverrides, MeshMode};
    use crate::db::{Database, MeshPick, QueueOutput};
    use voxel_queue::{Action, ItemProgress, next_action};

    fn later_flow(database: &Database) -> String {
        let flow = database
            .create_flow(&FlowDraft {
                name: "NPCs".to_owned(),
                template_id: None,
                path: None,
                items: vec![FlowItem {
                    id: "a".to_owned(),
                    prompt: "ferreiro".to_owned(),
                    count: 2,
                    mesh: MeshMode::Later,
                    template_id: None,
                    overrides: FlowOverrides::default(),
                }],
            })
            .unwrap();
        flow.id
    }

    #[test]
    fn escolhidas_viram_um_job_so_de_3d() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let flow_id = later_flow(&database);
        let job = database.enqueue_flow(&flow_id).unwrap();
        let item_id = job.items[0].id.clone();
        let mut progress = ItemProgress::pending(2, voxel_queue::MeshMode::Later);
        voxel_queue::record_images(&mut progress, 2);
        database
            .record_queue_success(
                &item_id,
                Action::Images,
                &[
                    QueueOutput {
                        path: "/tmp/a.png".to_owned(),
                        seed: Some(1),
                    },
                    QueueOutput {
                        path: "/tmp/b.png".to_owned(),
                        seed: Some(2),
                    },
                ],
                &progress,
            )
            .unwrap();
        database.finish_queue_job(&job.id).unwrap();
        let loaded = database.queue_job(&job.id).unwrap();
        assert!(waiting_later(&loaded.items[0]));
        assert_eq!(next_action(&Database::queue_progress(&loaded)), None);

        let mesh = database
            .enqueue_mesh_picks(
                &job.id,
                &[MeshPick {
                    item_id,
                    paths: vec!["/tmp/b.png".to_owned()],
                }],
            )
            .unwrap();
        assert!(mesh.name.contains("3D"));
        assert_eq!(mesh.items.len(), 1);
        assert_eq!(mesh.items[0].status, "mesh");
        assert_eq!(mesh.items[0].mesh, "all");
        assert_eq!(mesh.items[0].count, 1);
        assert_eq!(mesh.items[0].images, 1);
        assert_eq!(mesh.items[0].image_paths, ["/tmp/b.png"]);
        assert_eq!(
            next_action(&Database::queue_progress(&mesh)),
            Some((0, Action::Mesh))
        );
        assert!(
            database
                .enqueue_mesh_picks(
                    &job.id,
                    &[MeshPick {
                        item_id: mesh.items[0].id.clone(),
                        paths: vec!["/tmp/b.png".to_owned()],
                    }]
                )
                .is_err()
        );
    }
}
