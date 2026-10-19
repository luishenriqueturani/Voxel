fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "set_menu_labels",
            "get_settings",
            "update_settings",
            "hf_token_status",
            "set_hf_token",
            "download_model",
            "list_models",
            "import_model",
            "remove_model",
            "models_directory",
        ]),
    ))
    .expect("falha no build do Tauri");
}
