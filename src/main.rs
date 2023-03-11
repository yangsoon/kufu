use clap::Parser;
use fuser::MountOption;
use kufu::{args::Args, config::load, fuse::Fs, kube::watcher};
use tracing::info;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    let kufu_config = load(args.config_path).unwrap();

    let config = &kufu_config.kube_configs.unwrap()[0];
    let store = kufu::db::SledDb::new(&kufu_config.mount.data_path).unwrap();

    let mut watcher = watcher::Watcher::new(
        kufu_config.resources.unwrap().clone(),
        config,
        Box::new(store.clone()),
    )
    .await
    .unwrap();

    let client = watcher.client.clone();
    let options = vec![
        MountOption::FSName("kufu".to_string()),
        MountOption::AllowOther,
        MountOption::AutoUnmount,
    ];

    let kufu_fs = Fs::new(client, store, kufu_config.mount.path.clone());
    match kufu_fs.init() {
        Ok(()) => info!("success init kufu fs"),
        Err(e) => panic!("fail to init kufu fs, err: {:?}", e),
    }

    tokio::spawn(async move {
        watcher.build_api_pool().await.unwrap();
        watcher.watch().await.unwrap();
    });

    fuser::mount2(kufu_fs, &kufu_config.mount.path, &options).unwrap();
}
