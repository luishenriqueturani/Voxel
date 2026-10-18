/// Inicia a aplicação Tauri.
///
/// # Panics
///
/// Encerra o processo se o Tauri não conseguir inicializar a janela principal.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("falha ao iniciar a aplicação Tauri");
}
