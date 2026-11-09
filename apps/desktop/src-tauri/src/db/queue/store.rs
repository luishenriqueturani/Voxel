//! Enfileira, lê e grava o progresso de um job.

use rusqlite::{OptionalExtension, params};
use voxel_queue::{Action, ItemProgress};

use super::types::{QueueItem, QueueJob, QueueOutput, QueueResolved};
use super::{mesh_value, new_id, resolve_item, status_value};
use crate::db::{Database, DbError};
use crate::output::{unix_now, utc_timestamp};

impl Database {
    /// Cria um job com o snapshot dos itens do fluxo.
    ///
    /// # Errors
    ///
    /// Falha se o fluxo não existir, se não houver itens ou se o banco recusar a escrita.
    pub fn enqueue_flow(&self, flow_id: &str) -> Result<QueueJob, DbError> {
        let flow = self
            .list_flows()?
            .into_iter()
            .find(|entry| entry.id == flow_id)
            .ok_or(DbError::Missing)?;
        if flow.items.is_empty() {
            return Err(DbError::Invalid("fluxo sem itens"));
        }
        let template = match flow.template_id.as_deref() {
            Some(id) => Some(self.template(id)?),
            None => None,
        };
        let job_id = new_id();
        let now = utc_timestamp(unix_now());
        {
            let connection = self.lock()?;
            connection.execute(
                "INSERT INTO queue_jobs (id, flow_id, name, status, end_retry, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 'pending', 0, ?4, ?4)",
                params![job_id, flow.id, flow.name, now],
            )?;
            for (position, item) in flow.items.iter().enumerate() {
                let resolved = resolve_item(template.as_ref(), item);
                let payload = serde_json::to_string(&resolved)
                    .map_err(|_| DbError::Invalid("item de fila inválido"))?;
                let index =
                    i64::try_from(position).map_err(|_| DbError::Invalid("posição inválida"))?;
                connection.execute(
                    "INSERT INTO queue_items (
                        id, job_id, position, flow_item_id, status, mesh, count, images, meshes,
                        resolved, error_log, error_summary
                     ) VALUES (?1, ?2, ?3, ?4, 'pending', ?5, ?6, 0, 0, ?7, NULL, NULL)",
                    params![
                        new_id(),
                        job_id,
                        index,
                        item.id,
                        mesh_value(item.mesh),
                        item.count,
                        payload
                    ],
                )?;
            }
        }
        self.queue_job(&job_id)
    }

    /// Primeiro job ainda não encerrado. Pausado não entra: a fila espera o retomar.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn active_queue_job(&self) -> Result<Option<QueueJob>, DbError> {
        match self.unfinished_queue_job()? {
            Some(job) if job.status == "paused" => Ok(None),
            other => Ok(other),
        }
    }

    /// Primeiro job pendente, em execução ou pausado.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn unfinished_queue_job(&self) -> Result<Option<QueueJob>, DbError> {
        let id: Option<String> = {
            let connection = self.lock()?;
            let mut statement = connection.prepare(
                "SELECT id FROM queue_jobs
                 WHERE status IN ('pending', 'running', 'paused')
                 ORDER BY created_at, id
                 LIMIT 1",
            )?;
            let mut rows = statement.query([])?;
            match rows.next()? {
                Some(row) => Some(row.get(0)?),
                None => None,
            }
        };
        id.map(|value| self.queue_job(&value)).transpose()
    }

    /// Status do job mais recente, inclusive já encerrado.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn latest_queue_status(&self) -> Result<Option<String>, DbError> {
        let connection = self.lock()?;
        Ok(connection
            .query_row(
                "SELECT status FROM queue_jobs ORDER BY updated_at DESC, id DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// Job completo, com itens e caminhos.
    ///
    /// # Errors
    ///
    /// Falha se o job não existir.
    pub fn queue_job(&self, id: &str) -> Result<QueueJob, DbError> {
        let connection = self.lock()?;
        let (name, status, end_retry): (String, String, i64) = connection
            .query_row(
                "SELECT name, status, end_retry FROM queue_jobs WHERE id = ?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => DbError::Missing,
                other => DbError::from(other),
            })?;
        let mut statement = connection.prepare(
            "SELECT id, status, mesh, count, images, meshes, resolved, error_log, error_summary
             FROM queue_items
             WHERE job_id = ?1
             ORDER BY position, id",
        )?;
        let rows = statement.query_map([id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, Option<String>>(8)?,
            ))
        })?;
        let raw = rows.collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        let mut items = Vec::new();
        for (item_id, status, mesh, count, images, meshes, payload, error_log, error_summary) in raw
        {
            let resolved: QueueResolved = serde_json::from_str(&payload)
                .map_err(|_| DbError::Invalid("item de fila inválido"))?;
            let image_paths = image_paths_of(&connection, &item_id)?;
            items.push(QueueItem {
                prompt: resolved.prompt.clone(),
                id: item_id,
                status,
                count,
                mesh,
                images,
                meshes,
                resolved,
                error_log,
                error_summary,
                image_paths,
            });
        }
        Ok(QueueJob {
            id: id.to_owned(),
            name,
            status,
            end_retry: end_retry != 0,
            items,
        })
    }

    /// Marca o job como em execução.
    ///
    /// # Errors
    ///
    /// Falha se o job não existir.
    pub fn mark_queue_running(&self, id: &str) -> Result<(), DbError> {
        self.set_job_status(id, "running")
    }

    /// Encerra o job quando não resta passo pendente.
    ///
    /// # Errors
    ///
    /// Falha se o job não existir.
    pub fn finish_queue_job(&self, id: &str) -> Result<(), DbError> {
        self.set_job_status(id, "done")
    }

    /// Grava o progresso e as saídas de um passo bem-sucedido.
    ///
    /// # Errors
    ///
    /// Falha se o item não existir.
    pub fn record_queue_success(
        &self,
        item_id: &str,
        action: Action,
        outputs: &[QueueOutput],
        progress: &ItemProgress,
    ) -> Result<(), DbError> {
        let connection = self.lock()?;
        let kind = match action {
            Action::Images => "image",
            Action::Mesh => "mesh",
        };
        for (offset, output) in outputs.iter().enumerate() {
            let position = match action {
                Action::Images => progress
                    .images
                    .saturating_sub(u32::try_from(outputs.len()).unwrap_or(0)),
                Action::Mesh => progress.meshes.saturating_sub(1),
            }
            .saturating_add(u32::try_from(offset).unwrap_or(0));
            connection.execute(
                "INSERT INTO queue_outputs (id, item_id, kind, path, seed, position)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    new_id(),
                    item_id,
                    kind,
                    output.path,
                    output.seed,
                    i64::from(position)
                ],
            )?;
        }
        connection.execute(
            "UPDATE queue_items
             SET status = ?2, images = ?3, meshes = ?4, error_log = NULL, error_summary = NULL
             WHERE id = ?1",
            params![
                item_id,
                status_value(progress.status),
                i64::from(progress.images),
                i64::from(progress.meshes)
            ],
        )?;
        Ok(())
    }

    /// Marca o item como falho e guarda o caminho do log, sem abrir modal.
    ///
    /// # Errors
    ///
    /// Falha se o item não existir.
    pub fn record_queue_failure(
        &self,
        item_id: &str,
        log: &str,
        summary: &str,
    ) -> Result<(), DbError> {
        let changed = self.lock()?.execute(
            "UPDATE queue_items
             SET status = 'failed', error_log = ?2, error_summary = ?3
             WHERE id = ?1",
            params![item_id, log, summary],
        )?;
        if changed == 0 {
            return Err(DbError::Missing);
        }
        Ok(())
    }

    /// Atualiza o snapshot depois de um retry mais leve.
    ///
    /// # Errors
    ///
    /// Falha se o item não existir ou se o JSON for rejeitado.
    pub fn update_queue_resolved(
        &self,
        item_id: &str,
        resolved: &QueueResolved,
    ) -> Result<(), DbError> {
        let payload = serde_json::to_string(resolved)
            .map_err(|_| DbError::Invalid("item de fila inválido"))?;
        let changed = self.lock()?.execute(
            "UPDATE queue_items SET resolved = ?2 WHERE id = ?1",
            params![item_id, payload],
        )?;
        if changed == 0 {
            return Err(DbError::Missing);
        }
        Ok(())
    }

    pub(super) fn set_job_status(&self, id: &str, status: &str) -> Result<(), DbError> {
        let changed = self.lock()?.execute(
            "UPDATE queue_jobs SET status = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, status, utc_timestamp(unix_now())],
        )?;
        if changed == 0 {
            return Err(DbError::Missing);
        }
        Ok(())
    }
}

fn image_paths_of(
    connection: &rusqlite::Connection,
    item_id: &str,
) -> Result<Vec<String>, DbError> {
    let mut statement = connection.prepare(
        "SELECT path FROM queue_outputs WHERE item_id = ?1 AND kind = 'image' ORDER BY position, id",
    )?;
    let rows = statement.query_map([item_id], |row| row.get(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(DbError::from)
}

#[cfg(test)]
mod tests {
    use crate::db::Database;
    use crate::db::flows::{FlowDraft, FlowItem, FlowOverrides, MeshMode};
    use crate::db::templates::TemplateDraft;
    use voxel_queue::{Action, next_action};

    fn template() -> TemplateDraft {
        TemplateDraft {
            name: "Personagem".to_owned(),
            body: "{prompt}, T-pose".to_owned(),
            negative: "fundo".to_owned(),
            model_2d: "sd15".to_owned(),
            model_3d: "triposr".to_owned(),
            resolution: 512,
            aspect: "1:1".to_owned(),
            steps: 20,
            cfg: 7.0,
            seed: String::new(),
            count: 1,
            postprocess: false,
        }
    }

    fn flow_items() -> Vec<FlowItem> {
        vec![
            FlowItem {
                id: "a".to_owned(),
                prompt: "ferreiro".to_owned(),
                count: 1,
                mesh: MeshMode::All,
                overrides: FlowOverrides::default(),
            },
            FlowItem {
                id: "b".to_owned(),
                prompt: "guarda".to_owned(),
                count: 1,
                mesh: MeshMode::None,
                overrides: FlowOverrides::default(),
            },
        ]
    }

    #[test]
    fn enfileira_e_retoma_do_mesmo_ponto() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let template = database.create_template(&template()).unwrap();
        let flow = database
            .create_flow(&FlowDraft {
                name: "NPCs".to_owned(),
                template_id: Some(template.id),
                path: None,
                items: flow_items(),
            })
            .unwrap();
        let job = database.enqueue_flow(&flow.id).unwrap();
        assert_eq!(job.items.len(), 2);
        assert!(job.items[0].prompt.contains("ferreiro"));
        assert!(!job.end_retry);
        assert_eq!(database.active_queue_job().unwrap().unwrap().id, job.id);

        let progress = Database::queue_progress(&job);
        assert_eq!(next_action(&progress), Some((0, Action::Images)));
        let mut first = progress[0].clone();
        voxel_queue::record_images(&mut first, 1);
        database
            .record_queue_success(
                &job.items[0].id,
                Action::Images,
                &[super::QueueOutput {
                    path: "/tmp/a.png".to_owned(),
                    seed: Some(1),
                }],
                &first,
            )
            .unwrap();
        let resumed = database.queue_job(&job.id).unwrap();
        assert_eq!(resumed.items[0].status, "mesh");
        assert_eq!(resumed.items[0].image_paths, ["/tmp/a.png"]);
        assert_eq!(
            next_action(&Database::queue_progress(&resumed)),
            Some((0, Action::Mesh))
        );
    }

    #[test]
    fn falha_isolada_guarda_o_log_e_deixa_o_proximo() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let flow = database
            .create_flow(&FlowDraft {
                name: "NPCs".to_owned(),
                template_id: None,
                path: None,
                items: flow_items(),
            })
            .unwrap();
        let job = database.enqueue_flow(&flow.id).unwrap();
        database
            .record_queue_failure(&job.items[0].id, "/tmp/erro.log", "falhou")
            .unwrap();
        let loaded = database.queue_job(&job.id).unwrap();
        assert_eq!(loaded.items[0].status, "failed");
        assert_eq!(loaded.items[0].error_log.as_deref(), Some("/tmp/erro.log"));
        assert_eq!(
            next_action(&Database::queue_progress(&loaded)),
            Some((1, Action::Images))
        );
    }

    #[test]
    fn recusa_fluxo_sem_itens() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let flow = database
            .create_flow(&FlowDraft {
                name: "Vazio".to_owned(),
                template_id: None,
                path: None,
                items: vec![],
            })
            .unwrap();
        assert!(matches!(
            database.enqueue_flow(&flow.id),
            Err(crate::db::DbError::Invalid("fluxo sem itens"))
        ));
    }

    #[test]
    fn o_ultimo_job_guarda_o_status_depois_de_concluir() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let flow = database
            .create_flow(&FlowDraft {
                name: "NPCs".to_owned(),
                template_id: None,
                path: None,
                items: flow_items(),
            })
            .unwrap();
        assert_eq!(database.latest_queue_status().unwrap(), None);
        let job = database.enqueue_flow(&flow.id).unwrap();
        assert_eq!(
            database.latest_queue_status().unwrap().as_deref(),
            Some("pending")
        );
        database.set_job_status(&job.id, "done").unwrap();
        assert_eq!(
            database.latest_queue_status().unwrap().as_deref(),
            Some("done")
        );
    }
}
