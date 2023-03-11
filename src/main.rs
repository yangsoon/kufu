use clap::Parser;
use fuser::MountOption;
use kufu::{args::Args, config::load, fuse::Fs, kube::watcher};
use tracing::*;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let kufu_config = load(args.config_path).unwrap();

    let r = &kufu_config.resources.unwrap();
    let config = &kufu_config.kube_configs.unwrap()[0];

    let store = kufu::db::SledDb::new(&kufu_config.mount.data_path).unwrap();

    let mut watcher = watcher::Watcher::new(r, config, Box::new(store.clone()))
        .await
        .unwrap();

    let options = vec![MountOption::FSName("kufu".to_string())];
    match fuser::mount2(
        Fs::new(
            watcher.client.clone(),
            store,
            kufu_config.mount.path.clone(),
        ),
        &kufu_config.mount.path,
        &options,
    ) {
        Err(e) => {
            info!("fail mount kube-file-system: {:#?}", e)
        }
        Ok(_) => info!("mount file-system at: {:#?}", &kufu_config.mount.path),
    }
    watcher.build_api_pool().await.unwrap();
    watcher.watch().await.unwrap();
}
