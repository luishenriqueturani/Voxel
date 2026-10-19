mod db;
mod menu;
pub mod paths;

use std::collections::HashMap;

use db::{Database, SettingsUpdate, UserSettings};
use menu::{build_menu, default_labels};
use paths::AppPaths;
use tauri::{AppHandle, Emitter, Manager, State};
use voxel_logging::{AppLog, DEFAULT_RETENTION_DAYS};

/// Inicia a aplicação Tauri.
///
/// # Panics
///
/// Encerra o processo se o Tauri não conseguir inicializar a janela principal.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let paths = AppPaths::from_env()?;
            paths.create_all()?;
            let app_log = AppLog::start(&paths.state.join("logs"), DEFAULT_RETENTION_DAYS)?;
            app.manage(app_log);
            let database = Database::open(&paths.data.join("voxel.db"))?;
            database.migrate()?;
            app.manage(database);
            app.manage(paths);

            let menu = build_menu(app.handle(), &default_labels())?;
            app.set_menu(menu)?;
            app.on_menu_event(|app, event| {
                if event.id.0 == "file.quit" {
                    app.exit(0);
                    return;
                }
                let _ = app.emit("menu://action", event.id.0.clone());
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_menu_labels,
            get_settings,
            update_settings,
            power_limit_notice
        ])
        .run(tauri::generate_context!())
        .expect("falha ao iniciar a aplicação Tauri");
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn set_menu_labels(app: AppHandle, labels: HashMap<String, String>) -> Result<(), String> {
    let menu = build_menu(&app, &labels).map_err(|error| error.to_string())?;
    app.set_menu(menu).map_err(|error| error.to_string())?;
    Ok(())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn get_settings(database: State<'_, Database>) -> Result<UserSettings, String> {
    database.settings().map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn update_settings(
    database: State<'_, Database>,
    update: SettingsUpdate,
) -> Result<UserSettings, String> {
    database
        .update_settings(&update)
        .map_err(|error| error.to_string())
}

/// Nome da GPU que deve receber o aviso de power limit de 300 W, se houver.
#[tauri::command]
fn power_limit_notice() -> Option<String> {
    let machine = voxel_hardware::detect();
    let caps = voxel_hardware::read_power_caps(&machine.gpus);
    voxel_hardware::power_notice(&machine.gpus, &caps).map(|notice| notice.gpu_name)
}
