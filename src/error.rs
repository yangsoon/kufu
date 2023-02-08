use kube::config::KubeconfigError;
use kube::runtime::watcher;
use kube_client;
use kube_core::gvk;
use thiserror::Error;
use tokio::task;

#[derive(Error, Debug)]
pub enum Error {
    #[error("read kufu config failed: {0}")]
    ReadKubeConfigFail(String),

    #[error("load Kubeconfig failed: {0}")]
    LoadKubeconfigFail(#[from] KubeconfigError),

    #[error("build kube-client failed: {0}")]
    BuldKubeClientFail(#[from] kube_client::Error),

    #[error("parse gvk from TypeMeta failed: {0}")]
    ParseGVKFail(#[from] gvk::ParseGroupVersionError),

    #[error("watch event failed: {0}")]
    WatchEventFail(#[from] watcher::Error),

    #[error("tokio runtime join task error: {0}")]
    RuntimeJoinTaskFail(#[from] task::JoinError),
}
