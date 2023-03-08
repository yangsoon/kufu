use crate::{error::Error, Result};
use kube::{api::TypeMeta, config::Kubeconfig};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub mount: Mount,
    pub resources: Option<Vec<TypeMeta>>,

    #[serde(rename(serialize = "kube-configs", deserialize = "kube-configs"))]
    pub kube_configs: Option<Vec<KubeConfig>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Mount {
    pub data_path: String,
    pub path: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct KubeConfig {
    pub config_path: Option<String>,
    pub raw: Option<Kubeconfig>,
}

pub fn load<P>(config_path: P) -> Result<Config>
where
    P: AsRef<Path>,
{
    let kufu_config = match fs::read(config_path) {
        Ok(data) => data,
        Err(err) => {
            println!("read config fail: {}", err.to_string());
            return Err(Error::ReadKubeConfigFail(err.to_string()));
        }
    };
    let config: Config = match serde_yaml::from_slice(&kufu_config) {
        Ok(config) => config,
        Err(err) => {
            println!("decode config yaml fail: {}", err.to_string());
            return Err(Error::ReadKubeConfigFail(err.to_string()));
        }
    };
    return Ok(config);
}
