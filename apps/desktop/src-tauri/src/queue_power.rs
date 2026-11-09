//! Inibe suspensão enquanto a fila gera e dispara a ação ao ficar ociosa.

use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::db::Database;
use crate::inhibit::SleepInhibit;
use crate::paths::AppPaths;
use crate::queue_finish::apply_finish;
use crate::queue_run::{LiveExecutor, QueuePump, drive_until_idle};

/// Laço de fundo: retoma jobs pendentes e espera o próximo.
pub fn pump(app: &AppHandle) {
    let inhibit = SleepInhibit::default();
    loop {
        let database = app.state::<Database>().inner().clone();
        let paths = app.state::<AppPaths>().inner().clone();
        let executor = LiveExecutor::new(app, &database, &paths);
        let had_work = database.active_queue_job().ok().flatten().is_some();
        sync_inhibit(&inhibit, &database, had_work);
        let _ = drive_until_idle(&database, &executor);
        let still_active = database.active_queue_job().ok().flatten().is_some();
        sync_inhibit(&inhibit, &database, still_active);
        if had_work && !still_active {
            apply_finish(&database);
        }
        app.state::<QueuePump>().wait(Duration::from_secs(2));
    }
}

fn sync_inhibit(inhibit: &SleepInhibit, database: &Database, active: bool) {
    if should_inhibit(active, database.inhibit_sleep().ok().unwrap_or(true)) {
        inhibit.hold();
    } else {
        inhibit.release();
    }
}

/// Bloqueia o sono só com job ativo e a opção ligada.
#[must_use]
pub fn should_inhibit(active: bool, enabled: bool) -> bool {
    active && enabled
}

#[cfg(test)]
mod tests {
    use super::should_inhibit;

    #[test]
    fn so_inibe_com_fila_ativa_e_opcao_ligada() {
        assert!(should_inhibit(true, true));
        assert!(!should_inhibit(true, false));
        assert!(!should_inhibit(false, true));
        assert!(!should_inhibit(false, false));
    }
}
