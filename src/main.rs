use clap::Parser;
use kufu::{args::Args, config::load, kube::watcher};
use tracing::*;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    info!("build local file system for {}", args.cluster_name);
    let kufu_config = load(args.config_path).unwrap();

    let r = &kufu_config.resources.unwrap();
    let config = &kufu_config.kube_configs.unwrap()[0];

    let store = kufu::db::SledDb::new(kufu_config.mount.meta_path.unwrap());
    let mut watcher = watcher::Watcher::new(r, config, Box::new(store))
        .await
        .unwrap();
    watcher.build_api_pool().await.unwrap();
    watcher.watch().await.unwrap();
}
