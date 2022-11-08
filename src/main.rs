// use std::io;
use freedom_rust::{start_local, start_remote};
use tokio::io;
#[tokio::main]
async fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mode_default = String::from("local");
    let config_path_default = String::from("conf.local.yaml");
    // let config_path_default = String::from("conf.remote.yaml");
    let mode = args.get(1).unwrap_or(&mode_default);
    let config_path = args.get(2).unwrap_or(&config_path_default);

    println!("{} - {}", mode, config_path);
    if mode == "local" {
        start_local(config_path).await
    } else {
        start_remote(config_path).await
    }
}
