pub mod args;
pub mod config;
pub mod error;
pub mod kube;
pub mod utils;

pub type Result<T> = std::result::Result<T, error::Error>;
