//! Pausar, retomar, cancelar, reordenar pendentes e reabrir falhos.

use rusqlite::params;
use voxel_queue::{prepare_end_retry, reorder};

use super::status_value;
use super::types::QueueJob;
use crate::db::{Database, DbError};
use crate::output::{unix_now, utc_timestamp};

impl Database {
    /// Pausa o primeiro job ainda aberto. O passo em andamento deve ser interrompido à parte.
    ///
    /// # Errors
    ///
    /// Falha se não houver job aberto ou se o banco recusar a escrita.
    pub fn pause_queue(&self) -> Result<QueueJob, DbError> {
        let job = self
            .unfinished_queue_job()?
            .ok_or(DbError::Invalid("fila vazia"))?;
        if job.status != "paused" {
            self.set_job_status(&job.id, "paused")?;
        }
        self.queue_job(&job.id)
    }

    /// Retoma o job pausado.
    ///
    /// # Errors
    ///
    /// Falha se a fila não estiver pausada.
    pub fn resume_queue(&self) -> Result<QueueJob, DbError> {
        let job = self
            .unfinished_queue_job()?
            .ok_or(DbError::Invalid("fila vazia"))?;
        if job.status != "paused" {
            return Err(DbError::Invalid("fila não está pausada"));
        }
        self.set_job_status(&job.id, "pending")?;
        self.queue_job(&job.id)
    }

    /// Cancela todos os jobs abertos e os itens que ainda não terminaram.
    ///
    /// # Errors
    ///
    /// Falha se o banco recusar a escrita.
    pub fn cancel_queue(&self) -> Result<(), DbError> {
        let now = utc_timestamp(unix_now());
        let connection = self.lock()?;
        connection.execute(
            "UPDATE queue_items SET status = 'cancelled'
             WHERE status NOT IN ('done', 'failed', 'cancelled')
               AND job_id IN (
                    SELECT id FROM queue_jobs WHERE status IN ('pending', 'running', 'paused')
               )",
            [],
        )?;
        connection.execute(
            "UPDATE queue_jobs SET status = 'cancelled', updated_at = ?1
             WHERE status IN ('pending', 'running', 'paused')",
            params![now],
        )?;
        Ok(())
    }

    /// Troca a ordem dos itens pendentes. Os que já começaram ficam no lugar.
    ///
    /// `pending_ids` é a nova ordem só dos itens ainda não iniciados.
    ///
    /// # Errors
    ///
    /// Falha se o job não existir, se a lista não bater com os pendentes ou se o banco recusar.
    pub fn reorder_queue_items(
        &self,
        job_id: &str,
        pending_ids: &[String],
    ) -> Result<QueueJob, DbError> {
        let job = self.queue_job(job_id)?;
        if !matches!(job.status.as_str(), "pending" | "running" | "paused") {
            return Err(DbError::Invalid("fila encerrada"));
        }
        let progress = Self::queue_progress(&job);
        let mut from = Vec::new();
        for id in pending_ids {
            let index = job
                .items
                .iter()
                .position(|item| item.id == *id)
                .ok_or(DbError::Invalid("item de fila ausente"))?;
            from.push(index);
        }
        let order = reorder(&progress, &from).ok_or(DbError::Invalid("ordem inválida"))?;
        let now = utc_timestamp(unix_now());
        {
            let connection = self.lock()?;
            for (position, item_index) in order.iter().enumerate() {
                let item = job
                    .items
                    .get(*item_index)
                    .ok_or(DbError::Invalid("item de fila ausente"))?;
                let index =
                    i64::try_from(position).map_err(|_| DbError::Invalid("posição inválida"))?;
                connection.execute(
                    "UPDATE queue_items SET position = ?2 WHERE id = ?1",
                    params![item.id, index],
                )?;
            }
            connection.execute(
                "UPDATE queue_jobs SET updated_at = ?2 WHERE id = ?1",
                params![job_id, now],
            )?;
        }
        self.queue_job(job_id)
    }

    /// Reabre os falhos uma vez e marca que a passagem final já começou.
    ///
    /// # Errors
    ///
    /// Falha se o job não existir.
    pub fn apply_end_retry(&self, job_id: &str) -> Result<QueueJob, DbError> {
        let job = self.queue_job(job_id)?;
        let mut progress = Self::queue_progress(&job);
        if job.end_retry || !prepare_end_retry(&mut progress) {
            return Ok(job);
        }
        let now = utc_timestamp(unix_now());
        {
            let connection = self.lock()?;
            connection.execute(
                "UPDATE queue_jobs SET end_retry = 1, updated_at = ?2 WHERE id = ?1",
                params![job_id, now],
            )?;
            for (item, state) in job.items.iter().zip(progress.iter()) {
                connection.execute(
                    "UPDATE queue_items SET status = ?2 WHERE id = ?1",
                    params![item.id, status_value(state.status)],
                )?;
            }
        }
        self.queue_job(job_id)
    }
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::db::flows::{FlowDraft, FlowItem, FlowOverrides, MeshMode};

    fn items() -> Vec<FlowItem> {
        vec![
            FlowItem {
                id: "a".to_owned(),
                prompt: "ferreiro".to_owned(),
                count: 1,
                mesh: MeshMode::None,
                template_id: None,
                overrides: FlowOverrides::default(),
            },
            FlowItem {
                id: "b".to_owned(),
                prompt: "guarda".to_owned(),
                count: 1,
                mesh: MeshMode::None,
                template_id: None,
                overrides: FlowOverrides::default(),
            },
            FlowItem {
                id: "c".to_owned(),
                prompt: "aldeã".to_owned(),
                count: 1,
                mesh: MeshMode::None,
                template_id: None,
                overrides: FlowOverrides::default(),
            },
        ]
    }

    fn queued() -> (Database, String) {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let flow = database
            .create_flow(&FlowDraft {
                name: "NPCs".to_owned(),
                template_id: None,
                path: None,
                items: items(),
            })
            .unwrap();
        let job = database.enqueue_flow(&flow.id).unwrap();
        (database, job.id)
    }

    #[test]
    fn pausa_e_retoma_sem_pular_para_outro_job() {
        let (database, id) = queued();
        let paused = database.pause_queue().unwrap();
        assert_eq!(paused.status, "paused");
        assert!(database.active_queue_job().unwrap().is_none());
        assert_eq!(database.unfinished_queue_job().unwrap().unwrap().id, id);
        let resumed = database.resume_queue().unwrap();
        assert_eq!(resumed.status, "pending");
        assert_eq!(database.active_queue_job().unwrap().unwrap().id, id);
    }

    #[test]
    fn cancela_os_pendentes_e_mantem_os_concluidos() {
        let (database, id) = queued();
        let job = database.queue_job(&id).unwrap();
        let mut first = Database::queue_progress(&job);
        voxel_queue::record_images(&mut first[0], 1);
        database
            .record_queue_success(
                &job.items[0].id,
                voxel_queue::Action::Images,
                &[crate::db::QueueOutput {
                    path: "/tmp/a.png".to_owned(),
                    seed: Some(1),
                }],
                &first[0],
            )
            .unwrap();
        database.cancel_queue().unwrap();
        let loaded = database.queue_job(&id).unwrap();
        assert_eq!(loaded.status, "cancelled");
        assert_eq!(loaded.items[0].status, "done");
        assert_eq!(loaded.items[1].status, "cancelled");
        assert_eq!(loaded.items[2].status, "cancelled");
        assert!(database.active_queue_job().unwrap().is_none());
    }

    #[test]
    fn reordena_pendentes_sem_mover_quem_ja_comecou() {
        let (database, id) = queued();
        let job = database.queue_job(&id).unwrap();
        let mut first = Database::queue_progress(&job);
        voxel_queue::record_images(&mut first[0], 1);
        database
            .record_queue_success(
                &job.items[0].id,
                voxel_queue::Action::Images,
                &[crate::db::QueueOutput {
                    path: "/tmp/a.png".to_owned(),
                    seed: Some(1),
                }],
                &first[0],
            )
            .unwrap();
        let pending = [job.items[2].id.clone(), job.items[1].id.clone()];
        let reordered = database.reorder_queue_items(&id, &pending).unwrap();
        assert_eq!(reordered.items[0].prompt, "ferreiro");
        assert_eq!(reordered.items[1].prompt, "aldeã");
        assert_eq!(reordered.items[2].prompt, "guarda");
    }

    #[test]
    fn reabre_os_falhos_uma_vez() {
        let (database, id) = queued();
        let job = database.queue_job(&id).unwrap();
        database
            .record_queue_failure(&job.items[0].id, "/tmp/e.log", "vram")
            .unwrap();
        let retried = database.apply_end_retry(&id).unwrap();
        assert!(retried.end_retry);
        assert_eq!(retried.items[0].status, "pending");
        let again = database.apply_end_retry(&id).unwrap();
        assert_eq!(again.items[0].status, "pending");
        assert!(again.end_retry);
    }
}
