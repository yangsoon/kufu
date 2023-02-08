use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(long, default_value_t = String::from("local"))]
    pub cluster_name: String,

    #[arg(short,long, default_value_t = String::from("/Users/yangs/Project/Rust/kufu/test/config"))]
    pub config_path: String,
}
