use kube::{core::DynamicObject, discovery::Scope};
use sled::{Db, IVec};
use std::path::Path;
use std::sync::Arc;

use super::Store;
use crate::{ClusterObject, Result};

pub struct SledDb(Db);

impl SledDb {
    fn new(path: impl AsRef<Path>) -> SledDb {
        SledDb(sled::open(path).unwrap())
    }

    fn get_obj_full_key(cluster_obj: &ClusterObject) -> String {
        let kind = &cluster_obj.obj.types.as_ref().unwrap().kind;
        let name = cluster_obj.obj.metadata.name.as_ref().unwrap();
        match cluster_obj.caps.scope {
            Scope::Namespaced => format!(
                "{}/data/namespace/{}/{}/{}",
                cluster_obj.cluster,
                cluster_obj.obj.metadata.namespace.as_ref().unwrap(),
                kind,
                name
            ),
            Scope::Cluster => format!("{}/data/{}/{}", cluster_obj.cluster, kind, name),
        }
    }

    fn get_sub_obj_full_key() {}
}

impl Store for SledDb {
    fn set(&self, cluster_obj: &ClusterObject) -> Result<()> {
        let key = SledDb::get_obj_full_key(cluster_obj);
        let value: IVec = cluster_obj.try_into()?;
        self.0.insert(key, value)?;
        Ok(())
    }

    fn get(&self, cluster_obj: &ClusterObject) -> Result<Option<DynamicObject>> {
        let key = SledDb::get_obj_full_key(cluster_obj);
        let value = self.0.get(key)?;
        match value {
            Some(v) => Ok(Some(serde_yaml::from_slice(v.as_ref())?)),
            None => Ok(None),
        }
    }
}
