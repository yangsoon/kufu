use kube::{config::KubeconfigError, core::gvk, runtime::watcher};
use std::string::FromUtf8Error;
use thiserror::Error;
use tokio::task;

#[derive(Error, Debug)]
pub enum Error {
    #[error("read kufu config failed: {0}")]
    ReadKubeConfigFail(String),

    #[error("load Kubeconfig failed: {0}")]
    LoadKubeconfigFail(#[from] KubeconfigError),

    #[error("build kube-client failed: {0}")]
    BuildKubeClientFail(#[from] kube::error::Error),

    #[error("parse gvk from TypeMeta failed: {0}")]
    ParseGVKFail(#[from] gvk::ParseGroupVersionError),

    #[error("watch event failed: {0}")]
    WatchEventFail(#[from] watcher::Error),

    #[error("tokio runtime join task failed: {0}")]
    RuntimeJoinTaskFail(#[from] task::JoinError),

    #[error("store or get dynamicObject for sled failed: {0}")]
    StoreDynamicObjectFailed(#[from] sled::Error),

    #[error("serialize dynamicObject to yaml failed: {0}")]
    SerializeDynamicObject2Yaml(#[from] serde_yaml::Error),

    #[error("sled transaction failed: {0}")]
    SledTransactionError(#[from] sled::transaction::TransactionError),

    #[error("convert string to osstring failed: {0}")]
    ConvertOsStrError(#[from] core::convert::Infallible),

    #[error("look up inode attribute failed: {0}")]
    InodeAttrNotFound(u64),

    #[error("look up dentry attribute failed: {0}")]
    DentryAttrNotFound(u64),

    #[error("look up child: {1} entry from: parent: {0} failed")]
    ChildEntryNotFound(String, String),

    #[error("mock parent dir for {0} failed")]
    MockParentDirError(String),

    #[error("look up cluster object data {0} failed")]
    ClusterObjectDataNotFound(u64),

    #[error("covert ivec to string failed: {0}")]
    ConvertIVecToStringError(#[from] FromUtf8Error),
}
