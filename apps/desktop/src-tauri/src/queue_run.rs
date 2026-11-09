//! Executa a fila: um passo por vez, loga falha isolada, nunca abre o modal.

use std::sync::{Condvar, Mutex, PoisonError};
use std::time::Duration;

use tauri::AppHandle;

use crate::db::{Database, QueueItem, QueueJob, QueueOutput, QueueResolved};
use crate::generate::{self, GenerateRequest};
use crate::mesh::{self, MeshOptimize};
use crate::paths::AppPaths;
use voxel_queue::{
    Action, cancelled, job_finished, lighten_mesh, lighten_resolution, next_action, record_failure,
    record_images, record_mesh, vram_error,
};

const SIDE_MIN: u32 = 256;
const SIDE_MAX: u32 = 1024;
const SIDE_STEP: u32 = 64;

/// Acorda o laço da fila depois de enfileirar um fluxo.
#[derive(Default)]
pub struct QueuePump {
    lock: Mutex<()>,
    wake: Condvar,
}

impl QueuePump {
    /// Acorda o worker da fila.
    pub fn notify(&self) {
        self.wake.notify_one();
    }

    /// Espera o próximo enfileiramento ou o intervalo de varredura.
    pub fn wait(&self, timeout: Duration) {
        let guard = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        let _ = self.wake.wait_timeout(guard, timeout);
    }
}

/// Worker real: gera no sd-cli e na malha, e grava log sem abrir o modal.
pub struct LiveExecutor<'a> {
    app: &'a AppHandle,
    database: &'a Database,
    paths: &'a AppPaths,
}

impl<'a> LiveExecutor<'a> {
    /// Liga o worker ao app, ao banco e aos caminhos da sessão.
    #[must_use]
    pub fn new(app: &'a AppHandle, database: &'a Database, paths: &'a AppPaths) -> Self {
        Self {
            app,
            database,
            paths,
        }
    }
}

impl QueueExecutor for LiveExecutor<'_> {
    fn generate(
        &self,
        resolved: &QueueResolved,
        start: u32,
        remaining: u32,
    ) -> Result<Vec<QueueOutput>, String> {
        let (width, height) = frame_size(resolved.resolution, &resolved.aspect);
        let seed = parse_seed(&resolved.seed).map(|value| value.saturating_add(i64::from(start)));
        let image = generate::generate(
            self.app,
            self.database,
            self.paths,
            &GenerateRequest {
                prompt: resolved.prompt.clone(),
                negative_prompt: if resolved.negative.is_empty() {
                    None
                } else {
                    Some(resolved.negative.clone())
                },
                model_id: resolved.model_2d.clone(),
                width,
                height,
                steps: u32::try_from(resolved.steps).unwrap_or(20),
                cfg: resolved.cfg,
                seed,
                count: remaining.max(1),
                mode: String::new(),
                strength: 0.75,
                source_path: None,
                mask_png: None,
                backend: String::new(),
            },
        )?;
        Ok(image
            .images
            .into_iter()
            .map(|file| QueueOutput {
                path: file.path,
                seed: Some(file.seed),
            })
            .collect())
    }

    fn mesh(&self, image: &str, optimize: bool) -> Result<QueueOutput, String> {
        let options = optimize.then_some(MeshOptimize {
            triangles: 10_000,
            texture_size: 1024,
        });
        let files = mesh::from_image(self.app, self.paths, &[image.to_owned()], image, options)?;
        Ok(QueueOutput {
            path: files.glb_path,
            seed: None,
        })
    }

    fn write_log(&self, summary: &str, detail: &str) -> Result<String, String> {
        // Sem `error://show`: o relatório da fila lê estes arquivos depois.
        let body = format!("Fila\nItem: {summary}\n{detail}\n");
        crate::debug_log::write_debug(&self.paths.state.join("debug"), &body)
            .map(|path| path.display().to_string())
            .map_err(|error| error.to_string())
    }
}

fn parse_seed(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        trimmed.parse().ok()
    }
}

fn frame_size(resolution: i64, aspect: &str) -> (u32, u32) {
    let (width_ratio, height_ratio) = match aspect {
        "4:3" => (4, 3),
        "3:2" => (3, 2),
        "16:9" => (16, 9),
        "21:9" => (21, 9),
        "3:4" => (3, 4),
        "2:3" => (2, 3),
        "9:16" => (9, 16),
        "9:21" => (9, 21),
        _ => (1, 1),
    };
    let long_side = width_ratio.max(height_ratio);
    let resolution = u32::try_from(resolution).unwrap_or(512);
    (
        snap_side(resolution * width_ratio / long_side),
        snap_side(resolution * height_ratio / long_side),
    )
}

fn snap_side(value: u32) -> u32 {
    let clamped = value.clamp(SIDE_MIN, SIDE_MAX);
    let rounded = clamped.saturating_add(SIDE_STEP / 2) / SIDE_STEP * SIDE_STEP;
    rounded.clamp(SIDE_MIN, SIDE_MAX)
}

/// Quem gera imagem e malha. Os testes usam um dublê; o app usa o worker real.
pub trait QueueExecutor {
    /// Gera as imagens que ainda faltam neste item.
    ///
    /// # Errors
    ///
    /// Falha se o worker recusar o pedido.
    fn generate(
        &self,
        resolved: &QueueResolved,
        start: u32,
        remaining: u32,
    ) -> Result<Vec<QueueOutput>, String>;

    /// Gera a malha de uma variação já pronta.
    ///
    /// # Errors
    ///
    /// Falha se o worker 3D recusar o pedido.
    fn mesh(&self, image: &str, optimize: bool) -> Result<QueueOutput, String>;

    /// Grava o log da falha isolada, sem emitir o evento do modal.
    ///
    /// # Errors
    ///
    /// Falha se o arquivo de log não puder ser criado.
    fn write_log(&self, summary: &str, detail: &str) -> Result<String, String>;
}

enum Step {
    Outputs(Vec<QueueOutput>),
    Failed(String),
    Stopped,
}

/// Roda o próximo passo do job ativo. `false` quando a fila está ociosa.
///
/// # Errors
///
/// Falha se o banco recusar a escrita.
pub fn drive_step(database: &Database, executor: &dyn QueueExecutor) -> Result<bool, String> {
    let Some(job) = database
        .active_queue_job()
        .map_err(|error| error.to_string())?
    else {
        return Ok(false);
    };
    let mut progress = Database::queue_progress(&job);
    let Some((index, action)) = next_action(&progress) else {
        return close_or_retry(database, &job);
    };
    let item = job
        .items
        .get(index)
        .ok_or_else(|| "item de fila ausente".to_owned())?;
    database
        .mark_queue_running(&job.id)
        .map_err(|error| error.to_string())?;
    let current = progress.get(index).ok_or("item de fila ausente")?;
    match run_action(database, executor, item, action, current)? {
        Step::Stopped => return Ok(false),
        Step::Outputs(outputs) => {
            let added = u32::try_from(outputs.len()).unwrap_or(0);
            if let Some(state) = progress.get_mut(index) {
                match action {
                    Action::Images => record_images(state, added),
                    Action::Mesh => record_mesh(state),
                }
            }
            let state = progress.get(index).ok_or("item de fila ausente")?;
            database
                .record_queue_success(&item.id, action, &outputs, state)
                .map_err(|error| error.to_string())?;
        }
        Step::Failed(error) => {
            let log = executor.write_log(&item.prompt, &error)?;
            if let Some(state) = progress.get_mut(index) {
                record_failure(state);
            }
            database
                .record_queue_failure(&item.id, &log, &error)
                .map_err(|error| error.to_string())?;
        }
    }
    let updated = database
        .queue_job(&job.id)
        .map_err(|error| error.to_string())?;
    if job_finished(&Database::queue_progress(&updated), updated.end_retry) {
        database
            .finish_queue_job(&job.id)
            .map_err(|error| error.to_string())?;
    }
    Ok(true)
}

fn close_or_retry(database: &Database, job: &QueueJob) -> Result<bool, String> {
    if !job.end_retry {
        let retried = database
            .apply_end_retry(&job.id)
            .map_err(|error| error.to_string())?;
        if next_action(&Database::queue_progress(&retried)).is_some() {
            return Ok(true);
        }
    }
    database
        .finish_queue_job(&job.id)
        .map_err(|error| error.to_string())?;
    Ok(false)
}

fn run_action(
    database: &Database,
    executor: &dyn QueueExecutor,
    item: &QueueItem,
    action: Action,
    current: &voxel_queue::ItemProgress,
) -> Result<Step, String> {
    let mut resolved = item.resolved.clone();
    let mut remaining = current.count.saturating_sub(current.images).max(1);
    loop {
        let result = match action {
            Action::Images => executor.generate(&resolved, current.images, remaining),
            Action::Mesh => {
                let image = item
                    .image_paths
                    .get(usize::try_from(current.meshes).unwrap_or(0))
                    .ok_or_else(|| "imagem ausente".to_owned())?;
                executor
                    .mesh(image, resolved.postprocess)
                    .map(|output| vec![output])
            }
        };
        match result {
            Ok(outputs) => return Ok(Step::Outputs(outputs)),
            Err(error) if cancelled(&error) => return Ok(Step::Stopped),
            Err(error) if vram_error(&error) && lighten(&mut resolved, action, &mut remaining) => {
                database
                    .update_queue_resolved(&item.id, &resolved)
                    .map_err(|err| err.to_string())?;
            }
            Err(error) => return Ok(Step::Failed(error)),
        }
    }
}

fn lighten(resolved: &mut QueueResolved, action: Action, remaining: &mut u32) -> bool {
    match action {
        Action::Images => {
            let Some(next) = lighten_resolution(u32::try_from(resolved.resolution).unwrap_or(512))
            else {
                return false;
            };
            resolved.resolution = i64::from(next);
            *remaining = 1;
            true
        }
        Action::Mesh => {
            let Some(next) = lighten_mesh(resolved.postprocess) else {
                return false;
            };
            resolved.postprocess = next;
            true
        }
    }
}

/// Roda até acabar o job ativo.
///
/// # Errors
///
/// Falha se um passo não puder ser gravado.
pub fn drive_until_idle(database: &Database, executor: &dyn QueueExecutor) -> Result<(), String> {
    while drive_step(database, executor)? {}
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{QueueExecutor, drive_until_idle};
    use crate::db::flows::{FlowDraft, FlowItem, FlowOverrides, MeshMode};
    use crate::db::{Database, QueueOutput, QueueResolved};

    struct Fake {
        fail_on: Option<String>,
        vram_once: std::sync::Mutex<bool>,
        logs: std::sync::Mutex<Vec<String>>,
        steps: std::sync::Mutex<Vec<String>>,
    }

    impl Fake {
        fn new(fail_on: Option<&str>) -> Self {
            Self {
                fail_on: fail_on.map(ToOwned::to_owned),
                vram_once: std::sync::Mutex::default(),
                logs: std::sync::Mutex::default(),
                steps: std::sync::Mutex::default(),
            }
        }

        fn vram_once() -> Self {
            let fake = Self::new(None);
            *fake.vram_once.lock().expect("vram") = true;
            fake
        }
    }

    impl QueueExecutor for Fake {
        fn generate(
            &self,
            resolved: &QueueResolved,
            start: u32,
            remaining: u32,
        ) -> Result<Vec<QueueOutput>, String> {
            self.steps
                .lock()
                .expect("steps")
                .push(format!("img:{}:{}", resolved.prompt, resolved.resolution));
            if self.fail_on.as_deref() == Some(resolved.prompt.as_str()) {
                return Err("gpu".to_owned());
            }
            {
                let mut once = self.vram_once.lock().expect("vram");
                if *once {
                    *once = false;
                    return Err("CUDA out of memory".to_owned());
                }
            }
            Ok((0..remaining)
                .map(|offset| QueueOutput {
                    path: format!("{}-{}.png", resolved.prompt, start + offset),
                    seed: Some(i64::from(start + offset)),
                })
                .collect())
        }

        fn mesh(&self, image: &str, optimize: bool) -> Result<QueueOutput, String> {
            self.steps
                .lock()
                .expect("steps")
                .push(format!("mesh:{image}:{optimize}"));
            if optimize {
                let mut once = self.vram_once.lock().expect("vram");
                if *once {
                    *once = false;
                    return Err("CUDA out of memory".to_owned());
                }
            }
            Ok(QueueOutput {
                path: format!("{image}.glb"),
                seed: None,
            })
        }

        fn write_log(&self, summary: &str, detail: &str) -> Result<String, String> {
            let path = format!("/tmp/{summary}.log");
            self.logs
                .lock()
                .expect("logs")
                .push(format!("{path}:{detail}"));
            Ok(path)
        }
    }

    fn items() -> Vec<FlowItem> {
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
    fn intercala_imagens_e_3d_e_segue_o_proximo_item() {
        let (database, id) = queued();
        let fake = Fake::new(None);
        drive_until_idle(&database, &fake).unwrap();
        let job = database.queue_job(&id).unwrap();
        assert_eq!(job.status, "done");
        assert_eq!(job.items[0].status, "done");
        assert_eq!(job.items[1].status, "done");
        assert_eq!(
            fake.steps.lock().expect("steps").as_slice(),
            [
                "img:ferreiro:512",
                "mesh:ferreiro-0.png:false",
                "img:guarda:512"
            ]
        );
        assert!(fake.logs.lock().expect("logs").is_empty());
        assert!(database.active_queue_job().unwrap().is_none());
    }

    #[test]
    fn falha_isolada_grava_log_e_nao_para_a_fila() {
        let (database, id) = queued();
        let fake = Fake::new(Some("ferreiro"));
        drive_until_idle(&database, &fake).unwrap();
        let job = database.queue_job(&id).unwrap();
        assert_eq!(job.items[0].status, "failed");
        assert_eq!(job.items[0].error_log.as_deref(), Some("/tmp/ferreiro.log"));
        assert_eq!(job.items[1].status, "done");
        assert_eq!(job.status, "done");
        assert!(!fake.logs.lock().expect("logs").is_empty());
        assert_eq!(
            fake.steps.lock().expect("steps").as_slice(),
            ["img:ferreiro:512", "img:guarda:512", "img:ferreiro:512"]
        );
    }

    #[test]
    fn falta_de_vram_repete_com_resolucao_menor() {
        let (database, id) = queued();
        let fake = Fake::vram_once();
        drive_until_idle(&database, &fake).unwrap();
        let job = database.queue_job(&id).unwrap();
        assert_eq!(job.items[0].status, "done");
        assert_eq!(job.items[0].resolved.resolution, 256);
        assert_eq!(
            fake.steps.lock().expect("steps").as_slice()[..2],
            ["img:ferreiro:512", "img:ferreiro:256"]
        );
        assert!(fake.logs.lock().expect("logs").is_empty());
    }

    struct Stopper<'a> {
        database: &'a Database,
        pause: bool,
    }

    impl QueueExecutor for Stopper<'_> {
        fn generate(
            &self,
            _resolved: &QueueResolved,
            _start: u32,
            _remaining: u32,
        ) -> Result<Vec<QueueOutput>, String> {
            if self.pause {
                self.database.pause_queue().unwrap();
            } else {
                self.database.cancel_queue().unwrap();
            }
            Err("cancelado".to_owned())
        }

        fn mesh(&self, _image: &str, _optimize: bool) -> Result<QueueOutput, String> {
            Err("cancelado".to_owned())
        }

        fn write_log(&self, _summary: &str, _detail: &str) -> Result<String, String> {
            Ok("/tmp/stop.log".to_owned())
        }
    }

    #[test]
    fn pausar_nao_marca_o_item_como_falho() {
        let (database, id) = queued();
        drive_until_idle(
            &database,
            &Stopper {
                database: &database,
                pause: true,
            },
        )
        .unwrap();
        let job = database.queue_job(&id).unwrap();
        assert_eq!(job.status, "paused");
        assert_eq!(job.items[0].status, "pending");
        assert!(database.active_queue_job().unwrap().is_none());
        database.resume_queue().unwrap();
        let fake = Fake::new(None);
        drive_until_idle(&database, &fake).unwrap();
        assert_eq!(database.queue_job(&id).unwrap().status, "done");
    }

    #[test]
    fn cancelar_interrompe_os_pendentes() {
        let (database, id) = queued();
        drive_until_idle(
            &database,
            &Stopper {
                database: &database,
                pause: false,
            },
        )
        .unwrap();
        let job = database.queue_job(&id).unwrap();
        assert_eq!(job.status, "cancelled");
        assert_eq!(job.items[0].status, "cancelled");
        assert_eq!(job.items[1].status, "cancelled");
    }
}
