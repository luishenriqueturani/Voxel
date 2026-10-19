use std::path::PathBuf;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let openapi = std::fs::read_to_string(root.join("protocol/openapi.json"))
        .expect("ler protocol/openapi.json");
    let generated = voxel_protocol::generate(&openapi).expect("gerar os tipos");
    write(
        &root.join("crates/protocol/src/generated.rs"),
        &generated.rust,
    );
    write(
        &root.join("apps/desktop/src/protocol/types.ts"),
        &generated.typescript,
    );
    write(
        &root.join("workers/python/src/voxel_worker/protocol.py"),
        &generated.python,
    );
}

fn write(path: &std::path::Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("criar o diretório dos tipos");
    }
    std::fs::write(path, contents).expect("gravar os tipos gerados");
}
