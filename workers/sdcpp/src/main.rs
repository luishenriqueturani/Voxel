mod args;
mod run;
mod server;

use std::path::{Path, PathBuf};

#[tokio::main]
async fn main() {
    let token = match std::env::var("VOXEL_TOKEN") {
        Ok(token) if !token.is_empty() => token,
        _ => {
            eprintln!("VOXEL_TOKEN ausente");
            std::process::exit(2);
        }
    };
    let cli = std::env::var("VOXEL_SD_CLI")
        .ok()
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .or_else(discover_cli)
        .unwrap_or_else(|| PathBuf::from("sd-cli"));
    let backend = std::env::var("VOXEL_IMAGE_BACKEND").unwrap_or_else(|_| "vulkan".to_owned());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("escutar em 127.0.0.1");
    let port = listener.local_addr().expect("porta local").port();
    println!("VOXEL_READY {port}");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    server::serve(listener, token, cli, backend)
        .await
        .expect("servir o worker");
}

fn discover_cli() -> Option<PathBuf> {
    let mut starts = Vec::new();
    if let Ok(current) = std::env::current_dir() {
        starts.push(current);
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(parent) = executable.parent()
    {
        starts.push(parent.to_path_buf());
    }
    starts.into_iter().find_map(|start| find_up(&start))
}

fn find_up(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start);
    for _ in 0..8 {
        let dir = current?;
        let candidate = dir.join("workers/sdcpp/build/vulkan/bin/sd-cli");
        if candidate.is_file() {
            return Some(candidate);
        }
        current = dir.parent();
    }
    None
}
