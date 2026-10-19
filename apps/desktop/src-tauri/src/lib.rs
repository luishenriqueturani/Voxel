mod db;
mod generate;
mod menu;
pub mod paths;
mod probe;
mod setup;

use std::collections::HashMap;
use std::sync::Mutex;

use db::{Database, SettingsUpdate, UserSettings};
use menu::{build_menu, default_labels};
use paths::AppPaths;
use probe::discover;
use setup::{SetupChoices, SetupConfirmation, SetupSnapshot};
use tauri::{AppHandle, Emitter, Manager, State};
use voxel_logging::{AppLog, DEFAULT_RETENTION_DAYS};

#[derive(Default)]
struct SetupGate(Mutex<()>);

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
            app.manage(SetupGate::default());

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
            power_limit_notice,
            setup_status,
            run_setup,
            confirm_setup,
            reset_setup,
            image_backend_choice,
            set_image_backend,
            restore_image_backend,
            generate_image
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

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn setup_status(database: State<'_, Database>) -> Result<SetupSnapshot, String> {
    setup::status(&database)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn run_setup(
    app: AppHandle,
    database: State<'_, Database>,
    gate: State<'_, SetupGate>,
) -> Result<SetupSnapshot, String> {
    let _hold = gate.0.lock().map_err(|_| "setup ocupado".to_owned())?;
    if setup::status(&database)?.completed {
        return setup::status(&database);
    }
    setup::execute(&database, &discover(), |step| {
        let _ = app.emit("setup://progress", step);
    })
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn confirm_setup(
    database: State<'_, Database>,
    gate: State<'_, SetupGate>,
    choices: SetupChoices,
) -> Result<SetupConfirmation, String> {
    let _hold = gate.0.lock().map_err(|_| "setup ocupado".to_owned())?;
    setup::confirm(&database, &discover(), &choices)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn reset_setup(database: State<'_, Database>) -> Result<(), String> {
    database.reset_setup().map_err(|error| error.to_string())
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn image_backend_choice(
    database: State<'_, Database>,
) -> Result<generate::ImageBackendChoice, String> {
    generate::backend_choice(&database)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn set_image_backend(
    database: State<'_, Database>,
    backend: String,
) -> Result<generate::ImageBackendChoice, String> {
    generate::set_backend(&database, &backend)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn restore_image_backend(
    database: State<'_, Database>,
) -> Result<generate::ImageBackendChoice, String> {
    generate::restore_backend(&database)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn generate_image(
    app: AppHandle,
    database: State<'_, Database>,
    paths: State<'_, AppPaths>,
    request: generate::GenerateRequest,
) -> Result<generate::GeneratedImage, String> {
    generate::generate(&app, &database, &paths, &request)
}
