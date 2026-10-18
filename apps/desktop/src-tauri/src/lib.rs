mod menu;
pub mod paths;

use std::collections::HashMap;

use menu::{build_menu, default_labels};
use paths::AppPaths;
use tauri::{AppHandle, Emitter, Manager};

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
        .invoke_handler(tauri::generate_handler![set_menu_labels])
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
