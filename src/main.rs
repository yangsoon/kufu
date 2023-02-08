use clap::Parser;
use kufu::args::Args;
use kufu::config::load;
use tracing::*;

fn main() {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    info!("build local file system for {}", args.cluster_name);
    let kufu_config = load(args.config_path).unwrap();
}
