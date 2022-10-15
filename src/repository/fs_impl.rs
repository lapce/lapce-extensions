use crate::db::{self, prisma};

use super::*;
use std::{path::PathBuf, str::FromStr};
pub const DEFAULT_BASE_PATH: PathBuf = PathBuf::from_str("fs-registry");
pub struct FileSystemRepository {
    base_path: PathBuf,
}
impl Default for FileSystemRepository {
    fn default() -> Self {
        Self {
            base_path: DEFAULT_BASE_PATH,
        }
    }
}
impl FileSystemRepository {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }
    pub fn base_path(&self) -> PathBuf {
        self.base_path.clone()
    }
}
impl Repository for FileSystemRepository {
    fn publish(&mut self, volt_info: NewVoltInfo) -> Result<(), PublishError> {
        let db_connection = db::connect();
        let db_client = prisma::new_client().await;
        todo!()
    }

    fn create_version(
        &mut self,
        plugin_name: String,
        version: NewPluginVersion,
    ) -> Result<(), CreateVersionError> {
        todo!()
    }

    fn yank_version(
        &mut self,
        plugin_name: String,
        version: String,
    ) -> Result<(), YankVersionError> {
        todo!()
    }

    fn unpublish_plugin(&mut self, plugin_name: String) -> Result<(), UnpublishPluginError> {
        todo!()
    }
}
