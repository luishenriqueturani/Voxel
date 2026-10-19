mod sample;
mod server;

#[tokio::main]
async fn main() {
    let token = match std::env::var("VOXEL_TOKEN") {
        Ok(token) if !token.is_empty() => token,
        _ => {
            eprintln!("VOXEL_TOKEN ausente");
            std::process::exit(2);
        }
    };
    let scenario = std::env::var("VOXEL_SCENARIO").unwrap_or_else(|_| "ok".to_owned());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("escutar em 127.0.0.1");
    let port = listener.local_addr().expect("porta local").port();
    println!("VOXEL_READY {port}");
    let _ = std::io::Write::flush(&mut std::io::stdout());
    server::serve(listener, token, scenario)
        .await
        .expect("servir o worker falso");
}
