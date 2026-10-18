pub mod paths;

use paths::AppPaths;
use tauri::Manager;

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
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("falha ao iniciar a aplicação Tauri");
}
