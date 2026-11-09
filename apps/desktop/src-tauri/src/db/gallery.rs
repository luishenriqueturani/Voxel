//! Lista da galeria, com filtros por prompt, data, modelo, fluxo e tipo.

use serde::{Deserialize, Serialize};

use super::{Database, DbError};

const ENTRIES: &str = "
SELECT
    g.path AS path,
    g.prompt AS prompt,
    COALESCE(g.negative_prompt, '') AS negative_prompt,
    g.model_id AS model_id,
    g.steps AS steps,
    g.width AS width,
    g.height AS height,
    g.cfg AS cfg,
    g.seed AS seed,
    g.created_at AS created_at,
    CASE
        WHEN lower(g.path) LIKE '%.glb' OR lower(g.path) LIKE '%.gltf' THEN 'mesh'
        ELSE 'image'
    END AS kind,
    COALESCE(q.flow_id, '') AS flow_id,
    COALESCE(q.flow_name, '') AS flow_name
FROM generations g
LEFT JOIN (
    SELECT o.path AS path, MAX(j.flow_id) AS flow_id, MAX(f.name) AS flow_name
    FROM queue_outputs o
    JOIN queue_items i ON i.id = o.item_id
    JOIN queue_jobs j ON j.id = i.job_id
    LEFT JOIN flows f ON f.id = j.flow_id
    GROUP BY o.path
) q ON q.path = g.path
UNION ALL
SELECT
    o.path,
    COALESCE(json_extract(i.resolved, '$.prompt'), ''),
    COALESCE(json_extract(i.resolved, '$.negative'), ''),
    COALESCE(json_extract(i.resolved, '$.model3d'), ''),
    CAST(COALESCE(json_extract(i.resolved, '$.steps'), 0) AS INTEGER),
    CAST(COALESCE(json_extract(i.resolved, '$.resolution'), 0) AS INTEGER),
    CAST(COALESCE(json_extract(i.resolved, '$.resolution'), 0) AS INTEGER),
    COALESCE(json_extract(i.resolved, '$.cfg'), 0),
    COALESCE(o.seed, 0),
    j.updated_at,
    'mesh',
    COALESCE(j.flow_id, ''),
    COALESCE(f.name, '')
FROM queue_outputs o
JOIN queue_items i ON i.id = o.item_id
JOIN queue_jobs j ON j.id = i.job_id
LEFT JOIN flows f ON f.id = j.flow_id
WHERE o.kind = 'mesh'
  AND NOT EXISTS (SELECT 1 FROM generations g WHERE g.path = o.path)
";

/// Uma geração já gravada, sem a miniatura.
#[derive(Debug, Clone, PartialEq)]
pub struct GalleryRow {
    pub path: String,
    pub prompt: String,
    pub negative_prompt: String,
    pub model_id: String,
    pub steps: i64,
    pub width: i64,
    pub height: i64,
    pub cfg: f64,
    pub seed: i64,
    pub created_at: String,
    pub kind: String,
    pub flow_id: String,
    pub flow_name: String,
}

/// Recortes da pesquisa na galeria. Campo vazio ignora aquele critério.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GalleryFilter {
    pub prompt: String,
    pub model_id: String,
    pub flow_id: String,
    pub kind: String,
    pub from: String,
    pub to: String,
}

/// Fluxo que aparece em alguma geração.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryFlow {
    pub id: String,
    pub name: String,
}

/// Modelos e fluxos distintos para os filtros.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GalleryFacets {
    pub models: Vec<String>,
    pub flows: Vec<GalleryFlow>,
}

impl Database {
    /// Gerações mais novas primeiro, já filtradas.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn gallery(&self, filter: &GalleryFilter, limit: i64) -> Result<Vec<GalleryRow>, DbError> {
        let sql = format!(
            "WITH entries AS ({ENTRIES})
             SELECT path, prompt, negative_prompt, model_id, steps, width, height, cfg, seed,
                    created_at, kind, flow_id, flow_name
             FROM entries
             WHERE (?1 = '' OR instr(lower(prompt), lower(?1)) > 0)
               AND (?2 = '' OR model_id = ?2)
               AND (?3 = '' OR flow_id = ?3)
               AND (?4 = '' OR kind = ?4)
               AND (?5 = '' OR substr(created_at, 1, 10) >= ?5)
               AND (?6 = '' OR substr(created_at, 1, 10) <= ?6)
             ORDER BY created_at DESC, path DESC
             LIMIT ?7"
        );
        let connection = self.lock()?;
        let mut statement = connection.prepare(&sql)?;
        let rows = statement
            .query_map(
                rusqlite::params![
                    filter.prompt,
                    filter.model_id,
                    filter.flow_id,
                    filter.kind,
                    filter.from,
                    filter.to,
                    limit
                ],
                map_gallery,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Modelos e fluxos que já saíram na galeria.
    ///
    /// # Errors
    ///
    /// Falha se a leitura do banco falhar.
    pub fn gallery_facets(&self) -> Result<GalleryFacets, DbError> {
        let models_sql = format!(
            "WITH entries AS ({ENTRIES}) SELECT DISTINCT model_id FROM entries WHERE model_id != '' ORDER BY model_id"
        );
        let flows_sql = format!(
            "WITH entries AS ({ENTRIES})
             SELECT DISTINCT flow_id, flow_name FROM entries
             WHERE flow_id != ''
             ORDER BY flow_name, flow_id"
        );
        let connection = self.lock()?;
        let mut models_statement = connection.prepare(&models_sql)?;
        let models = models_statement
            .query_map([], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        drop(models_statement);
        let mut flows_statement = connection.prepare(&flows_sql)?;
        let flows = flows_statement
            .query_map([], |row| {
                Ok(GalleryFlow {
                    id: row.get(0)?,
                    name: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(GalleryFacets { models, flows })
    }
}

fn map_gallery(row: &rusqlite::Row<'_>) -> rusqlite::Result<GalleryRow> {
    Ok(GalleryRow {
        path: row.get(0)?,
        prompt: row.get(1)?,
        negative_prompt: row.get(2)?,
        model_id: row.get(3)?,
        steps: row.get(4)?,
        width: row.get(5)?,
        height: row.get(6)?,
        cfg: row.get(7)?,
        seed: row.get(8)?,
        created_at: row.get(9)?,
        kind: row.get(10)?,
        flow_id: row.get(11)?,
        flow_name: row.get(12)?,
    })
}

#[cfg(test)]
mod tests {
    use super::{Database, GalleryFilter};
    use crate::db::flows::{FlowDraft, FlowItem, MeshMode};
    use crate::db::{GenerationRecord, QueueOutput};
    use voxel_queue::{Action, record_images, record_mesh};

    fn record(prompt: &str, seed: i64, path: &str, created_at: &str) -> GenerationRecord {
        GenerationRecord {
            path: path.to_owned(),
            prompt: prompt.to_owned(),
            negative_prompt: None,
            model_id: "sd15".to_owned(),
            backend: "vulkan".to_owned(),
            seed,
            steps: 25,
            width: 512,
            height: 512,
            cfg: 7.0,
            elapsed_ms: 1,
            created_at: created_at.to_owned(),
        }
    }

    fn listed(database: &Database, filter: &GalleryFilter) -> Vec<String> {
        database
            .gallery(filter, 20)
            .unwrap()
            .into_iter()
            .map(|row| row.prompt)
            .collect()
    }

    #[test]
    fn a_galeria_vem_da_mais_nova() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        database
            .record_generation(&record(
                "primeira",
                1,
                "/tmp/primeira.png",
                "2026-10-20T00:00:00Z",
            ))
            .unwrap();
        database
            .record_generation(&record(
                "segunda",
                2,
                "/tmp/segunda.png",
                "2026-10-21T00:00:00Z",
            ))
            .unwrap();
        let rows = database.gallery(&GalleryFilter::default(), 10).unwrap();
        assert_eq!(rows[0].prompt, "segunda");
        assert_eq!(rows[1].seed, 1);
        assert_eq!(rows[0].kind, "image");
    }

    #[test]
    fn filtra_prompt_modelo_data_e_tipo() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        database
            .record_generation(&record(
                "um gato",
                1,
                "/tmp/gato.png",
                "2026-10-20T00:00:00Z",
            ))
            .unwrap();
        database
            .record_generation(&record(
                "uma casa",
                2,
                "/tmp/casa.png",
                "2026-11-01T00:00:00Z",
            ))
            .unwrap();
        let mut mesh = record("um gato", 3, "/tmp/gato.glb", "2026-11-02T00:00:00Z");
        mesh.model_id = "triposr".to_owned();
        database.record_generation(&mesh).unwrap();

        assert_eq!(
            listed(
                &database,
                &GalleryFilter {
                    prompt: "gato".to_owned(),
                    ..GalleryFilter::default()
                }
            ),
            vec!["um gato", "um gato"]
        );
        assert_eq!(
            listed(
                &database,
                &GalleryFilter {
                    model_id: "triposr".to_owned(),
                    ..GalleryFilter::default()
                }
            ),
            vec!["um gato"]
        );
        assert_eq!(
            listed(
                &database,
                &GalleryFilter {
                    from: "2026-11-01".to_owned(),
                    to: "2026-11-01".to_owned(),
                    ..GalleryFilter::default()
                }
            ),
            vec!["uma casa"]
        );
        let meshes = database
            .gallery(
                &GalleryFilter {
                    kind: "mesh".to_owned(),
                    ..GalleryFilter::default()
                },
                10,
            )
            .unwrap();
        assert_eq!(meshes.len(), 1);
        assert_eq!(meshes[0].path, "/tmp/gato.glb");
        assert_eq!(
            listed(
                &database,
                &GalleryFilter {
                    kind: "image".to_owned(),
                    ..GalleryFilter::default()
                }
            )
            .len(),
            2
        );
    }

    #[test]
    fn filtra_pelo_fluxo_e_mostra_a_malha_da_fila() {
        let database = Database::open_in_memory().unwrap();
        database.migrate().unwrap();
        let flow = database
            .create_flow(&FlowDraft {
                name: "NPCs".to_owned(),
                template_id: None,
                path: None,
                items: vec![FlowItem {
                    id: "a".to_owned(),
                    prompt: "ferreiro".to_owned(),
                    count: 1,
                    mesh: MeshMode::All,
                    template_id: None,
                    overrides: crate::db::flows::FlowOverrides::default(),
                }],
            })
            .unwrap();
        let job = database.enqueue_flow(&flow.id).unwrap();
        database
            .record_generation(&record(
                "ferreiro",
                1,
                "/tmp/ferreiro.png",
                "2026-11-03T00:00:00Z",
            ))
            .unwrap();
        let mut progress = Database::queue_progress(&job);
        record_images(&mut progress[0], 1);
        database
            .record_queue_success(
                &job.items[0].id,
                Action::Images,
                &[QueueOutput {
                    path: "/tmp/ferreiro.png".to_owned(),
                    seed: Some(1),
                }],
                &progress[0],
            )
            .unwrap();
        record_mesh(&mut progress[0]);
        database
            .record_queue_success(
                &job.items[0].id,
                Action::Mesh,
                &[QueueOutput {
                    path: "/tmp/ferreiro.glb".to_owned(),
                    seed: Some(1),
                }],
                &progress[0],
            )
            .unwrap();

        let rows = database
            .gallery(
                &GalleryFilter {
                    flow_id: flow.id.clone(),
                    ..GalleryFilter::default()
                },
                10,
            )
            .unwrap();
        assert!(rows.iter().any(|row| row.path.ends_with("ferreiro.png")));
        assert!(rows.iter().any(|row| row.path.ends_with("ferreiro.glb")));
        assert!(rows.iter().all(|row| row.flow_name == "NPCs"));

        let facets = database.gallery_facets().unwrap();
        assert!(facets.models.iter().any(|model| model == "sd15"));
        assert!(facets.models.iter().any(|model| model == "triposr"));
        assert_eq!(facets.flows[0].name, "NPCs");
        assert_eq!(facets.flows[0].id, flow.id);
    }
}
