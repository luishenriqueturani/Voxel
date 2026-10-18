fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "set_menu_labels",
            "get_settings",
            "update_settings",
        ]),
    ))
    .expect("falha no build do Tauri");
}
