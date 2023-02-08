use crate::{error::Error, Result};
use kube::api::TypeMeta;
use kube::config::Kubeconfig;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    mount: Mount,
    resources: Option<Vec<TypeMeta>>,

    #[serde(rename(serialize = "kube-configs", deserialize = "kube-configs"))]
    kube_configs: Option<Vec<KubeConfig>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct Mount {
    data_path: Option<String>,
    meta_path: Option<String>,
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
