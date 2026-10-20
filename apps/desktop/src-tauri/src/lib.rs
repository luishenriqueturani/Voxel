mod background;
mod db;
mod generate;
mod history;
mod image_job;
mod live_job;
mod menu;
mod models;
mod output;
pub mod paths;
mod probe;
mod python_env;
mod setup;
mod status;

use std::collections::HashMap;
use std::sync::Mutex;

use background::{open_edit_image, remove_background};
use db::{Database, SettingsUpdate, UserSettings};
use history::{list_gallery, list_prompt_images, list_recent_prompts, set_prompt_favorite};
use menu::{build_menu, default_labels};
use paths::AppPaths;
use probe::discover;
use setup::{SetupChoices, SetupConfirmation, SetupSnapshot};
use tauri::{AppHandle, Emitter, Manager, State};
use voxel_logging::{AppLog, DEFAULT_RETENTION_DAYS};
use voxel_models::{Catalog, Kind};

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
        .plugin(tauri_plugin_dialog::init())
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
            app.manage(live_job::LiveJobs::default());
            app.manage(python_env::Environment::default());

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
            generate_image,
            cancel_generation,
            generation_defaults,
            open_edit_image,
            remove_background,
            list_recent_prompts,
            list_prompt_images,
            set_prompt_favorite,
            list_gallery,
            machine_status,
            copy_generation,
            hf_token_status,
            set_hf_token,
            download_model,
            list_models,
            import_model,
            remove_model,
            models_directory,
            python_env::python_environment_status,
            python_env::note_3d_intent,
            python_env::await_python_environment,
            python_env::reinstall_python_environment,
            python_env::remove_python_environment
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

/// Fora da thread da janela, para a prévia pintar a cada passo.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
async fn generate_image(
    app: AppHandle,
    database: State<'_, Database>,
    paths: State<'_, AppPaths>,
    request: generate::GenerateRequest,
) -> Result<generate::GeneratedImage, String> {
    let database = database.inner().clone();
    let paths = paths.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        generate::generate(&app, &database, &paths, &request)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn generation_defaults(database: State<'_, Database>) -> voxel_hardware::GenerationDefaults {
    generate::generation_defaults(&database)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn cancel_generation(jobs: State<'_, live_job::LiveJobs>) {
    jobs.request_stop();
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn machine_status(database: State<'_, Database>) -> Result<status::MachineStatus, String> {
    status::read_status(&database)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn copy_generation(source: String, destination: String) -> Result<(), String> {
    output::copy_generation(
        std::path::Path::new(&source),
        std::path::Path::new(&destination),
    )
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn hf_token_status(database: State<'_, Database>) -> Result<bool, String> {
    models::token_configured(&database)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn set_hf_token(database: State<'_, Database>, token: String) -> Result<bool, String> {
    models::store_token(&database, &token)
}

/// Fora da thread da janela, para o progresso pintar enquanto o arquivo chega.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
async fn download_model(
    app: AppHandle,
    database: State<'_, Database>,
    paths: State<'_, AppPaths>,
    model_id: String,
    confirmed: bool,
) -> Result<String, String> {
    let database = database.inner().clone();
    let paths = paths.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let saved = models::download(&app, &database, &paths, &model_id, confirmed)?;
        if Catalog::bundled().ok().is_some_and(|catalog| {
            catalog
                .get(&model_id)
                .is_some_and(|model| model.kind == Kind::Mesh)
        }) {
            app.state::<python_env::Environment>()
                .note(&app, python_env::Intent::ModelDownload);
        }
        Ok(saved)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn list_models(paths: State<'_, AppPaths>) -> Result<Vec<models::ListedModel>, String> {
    let mut listed = models::list_models(&paths.data.join("models"), &models::current_facts())?;
    if discover().model.is_some()
        && let Some(model) = listed.iter_mut().find(|model| model.id == "sd15")
    {
        model.installed = true;
    }
    Ok(listed)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn import_model(paths: State<'_, AppPaths>, path: String) -> Result<models::ListedModel, String> {
    let facts = models::current_facts();
    models::import_model(
        &paths.data.join("models"),
        std::path::Path::new(&path),
        &facts,
    )
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn remove_model(paths: State<'_, AppPaths>, filename: String) -> Result<(), String> {
    models::remove_model(&paths.data.join("models"), &filename)
}

#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
fn models_directory(paths: State<'_, AppPaths>) -> Result<String, String> {
    models::ensure_models_dir(&paths.data.join("models")).map(|path| path.display().to_string())
}
