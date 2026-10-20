//! Ambiente Python isolado, instalado só quando alguém pede 3D.

mod gate;
mod plan;
mod probe;
mod progress;
mod run;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use gate::Gate;
pub use plan::Intent;
pub(crate) use plan::{Layout, process_env};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::paths::AppPaths;

const EVENT: &str = "python://progress";

/// Estado mostrado na barra e nas configurações.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PythonProgress {
    pub phase: String,
    pub percent: Option<u8>,
    pub received_bytes: Option<u64>,
    pub total_bytes: Option<u64>,
    pub backend: Option<String>,
    pub detail: Option<String>,
}

impl PythonProgress {
    pub(crate) fn absent() -> Self {
        Self::bare("absent")
    }

    pub(crate) fn preparing(
        percent: Option<u8>,
        received: Option<u64>,
        total: Option<u64>,
    ) -> Self {
        Self {
            phase: "preparing".to_owned(),
            percent,
            received_bytes: received,
            total_bytes: total,
            backend: None,
            detail: None,
        }
    }

    pub(crate) fn testing() -> Self {
        Self::bare("testing")
    }

    pub(crate) fn ready(backend: &str) -> Self {
        Self {
            backend: Some(backend.to_owned()),
            ..Self::bare("ready")
        }
    }

    pub(crate) fn failed(message: &str) -> Self {
        Self {
            detail: Some(message.to_owned()),
            ..Self::bare("failed")
        }
    }

    fn bare(phase: &str) -> Self {
        Self {
            phase: phase.to_owned(),
            percent: None,
            received_bytes: None,
            total_bytes: None,
            backend: None,
            detail: None,
        }
    }

    pub(crate) fn is_working(&self) -> bool {
        self.phase == "preparing" || self.phase == "testing"
    }
}

#[derive(Deserialize)]
struct StoredBackend {
    backend: String,
}

/// Instalação em segundo plano e a fila de quem espera por ela.
#[derive(Default)]
pub struct Environment {
    gate: Gate,
    hydrated: AtomicBool,
    child: AtomicU32,
}

impl Environment {
    pub fn note(&self, app: &AppHandle, intent: Intent) {
        let _ = intent;
        let paths = app.state::<AppPaths>();
        self.hydrate(&paths);
        if let Some(job) = self.gate.start() {
            Self::spawn(app, false, job);
        }
    }

    pub fn reinstall(&self, app: &AppHandle) {
        run::stop_child(&self.child);
        let job = self.gate.restart();
        Self::spawn(app, true, job);
    }

    /// Apaga o interpretador, o venv e o cache do `uv`.
    ///
    /// # Errors
    ///
    /// Falha se a instalação ainda está andando ou se a pasta não puder ser removida.
    pub fn remove(&self, paths: &AppPaths) -> Result<(), String> {
        if self.gate.get().is_working() {
            return Err("instalação em andamento".to_owned());
        }
        let layout = Layout::new(&paths.data, &paths.cache);
        if layout.root.exists() {
            std::fs::remove_dir_all(&layout.root).map_err(|error| error.to_string())?;
        }
        if layout.cache.exists() {
            std::fs::remove_dir_all(&layout.cache).map_err(|error| error.to_string())?;
        }
        self.gate.set(PythonProgress::absent());
        Ok(())
    }

    pub fn status(&self, paths: &AppPaths) -> PythonProgress {
        self.hydrate(paths);
        self.gate.get()
    }

    /// Espera o ambiente. Se a instalação ainda não começou, ela começa agora.
    ///
    /// # Errors
    ///
    /// Falha se o teste real de todos os backends falhar.
    pub fn wait_ready(&self, app: &AppHandle) -> Result<(), String> {
        self.note(app, Intent::Panel);
        let progress = self.gate.wait();
        match progress.phase.as_str() {
            "ready" => Ok(()),
            "failed" => Err(progress
                .detail
                .unwrap_or_else(|| "ambiente 3D falhou".to_owned())),
            _ => Err("ambiente 3D ausente".to_owned()),
        }
    }

    fn hydrate(&self, paths: &AppPaths) {
        if self.hydrated.swap(true, Ordering::SeqCst) {
            return;
        }
        let layout = Layout::new(&paths.data, &paths.cache);
        if !layout.python().is_file() {
            return;
        }
        let Ok(text) = std::fs::read_to_string(&layout.marker) else {
            return;
        };
        if let Ok(stored) = serde_json::from_str::<StoredBackend>(&text) {
            self.gate
                .set_if_absent(PythonProgress::ready(&stored.backend));
        }
    }

    fn spawn(app: &AppHandle, wipe: bool, job: u64) {
        let app = app.clone();
        std::thread::spawn(move || {
            let env = app.state::<Environment>();
            let paths = app.state::<AppPaths>().inner().clone();
            let outcome = run::install(
                &paths,
                wipe,
                &env.child,
                job,
                &|| env.gate.current_job(),
                |progress| {
                    if env.gate.publish(job, progress.clone()) {
                        let _ = app.emit(EVENT, &progress);
                    }
                },
            );
            if env.gate.current_job() != job {
                return;
            }
            let progress = match outcome {
                Ok(backend) => PythonProgress::ready(&backend),
                Err(error) if error == run::INTERRUPTED => return,
                Err(error) => PythonProgress::failed(&error),
            };
            if env.gate.publish(job, progress.clone()) {
                let _ = app.emit(EVENT, &progress);
            }
        });
    }
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn python_environment_status(
    env: State<'_, Environment>,
    paths: State<'_, AppPaths>,
) -> PythonProgress {
    env.status(&paths)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn note_3d_intent(
    app: AppHandle,
    env: State<'_, Environment>,
    reason: String,
) -> Result<(), String> {
    let intent = Intent::parse(&reason).ok_or("intenção desconhecida")?;
    env.note(&app, intent);
    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub async fn await_python_environment(app: AppHandle) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || app.state::<Environment>().wait_ready(&app))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn reinstall_python_environment(app: AppHandle, env: State<'_, Environment>) {
    env.reinstall(&app);
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn remove_python_environment(
    env: State<'_, Environment>,
    paths: State<'_, AppPaths>,
) -> Result<(), String> {
    env.remove(&paths)
}
