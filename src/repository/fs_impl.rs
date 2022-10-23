use crate::db::{self, prisma};

use super::*;
use lazy_static::*;
use sha1::{Digest, Sha1};
use std::{cmp::Ordering, path::PathBuf, str::FromStr, fmt::Display};

#[cfg(test)]
mod tests;
lazy_static! {
    pub static ref DEFAULT_BASE_PATH: PathBuf = PathBuf::from_str("fs-registry").unwrap();
}
pub struct FileSystemRepository {
    base_path: PathBuf,
}

impl Default for FileSystemRepository {
    fn default() -> Self {
        Self {
            base_path: DEFAULT_BASE_PATH.to_path_buf(),
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
    fn version_path<P: AsRef<str> + Display, V: AsRef<str> + Display>(&self, plugin_name: P, version: V) -> PathBuf {
        let mut base_dir = self.base_path();
        base_dir.push("versions");
        std::fs::create_dir_all(&base_dir).unwrap();
        base_dir.push(format!("{}-{}.tar.gz", plugin_name, version));
        base_dir
    }
    fn icons_path(&self) -> PathBuf {
        let mut base_dir = self.base_path();
        base_dir.push("icons");
        std::fs::create_dir_all(&base_dir).unwrap();
        base_dir
    }
    fn remove_version(&self, plugin_name: String, version: String) -> std::io::Result<()> {
        let mut path = self.base_path();
        path.push("versions");
        path.push(format!("{}-{}", &plugin_name, &version));
        std::fs::remove_dir_all(path)?;
        Ok(())
    }
}
#[async_trait]
impl Repository for FileSystemRepository {
    async fn get_plugin_version_tar(
        &mut self,
        name: String,
        version: String,
    ) -> Result<Vec<u8>, GetResourceError> {
        let path = self.version_path(name, version);
        if path.try_exists().map_err(|_| GetResourceError::IoError)? {
            std::fs::read(path).map_err(|_| GetResourceError::IoError)
        } else {
            Err(GetResourceError::NotFound)
        }
    }
    async fn get_plugin_icon(&mut self, name: String) -> Result<Vec<u8>, GetResourceError> {
        let mut path = self.icons_path();
        path.push(&name);
        if path.try_exists().map_err(|_| GetResourceError::IoError)? {
            std::fs::read(path).map_err(|_| GetResourceError::IoError)
        } else {
            Err(GetResourceError::NotFound)
        }
    }
    async fn save_icon(&mut self, plugin_name: String, icon: &[u8]) -> Result<(), PublishError> {
        if validate_icon(icon).is_some() {
            return Err(PublishError::InvalidIcon);
        }
        let mut icon_path = self.icons_path();
        std::fs::create_dir_all(icon_path.clone()).map_err(|_| PublishError::IoError)?;
        icon_path.push(&plugin_name);
        std::fs::write(icon_path, icon).map_err(|_| PublishError::IoError)?;
        Ok(())
    }
    async fn create_version(
        &mut self,
        plugin_name: String,
        version: NewPluginVersion,
    ) -> Result<(), CreateVersionError> {
        let db_client = db::connect().await.map_err(|e| {
            eprintln!("Failed to connect to the database: {:#?}", e);
            CreateVersionError::DatabaseError
        })?;
        let convert_semver_err = || {
            eprintln!("Tried to release a invalid version: {}", &version.version);
            CreateVersionError::InvalidSemVer
        };
        #[inline]
        fn parse_semver(version: &str) -> semver::Version {
            semver::Version::from_str(version).unwrap()
        }
        let semversion =
            semver::Version::from_str(&version.version).map_err(|_| convert_semver_err())?;
        match db_client
            .plugin()
            .find_unique(prisma::plugin::name::equals(plugin_name.clone()))
            .with(prisma::plugin::versions::fetch(vec![]))
            .exec()
            .await
        {
            Ok(None) => Err(CreateVersionError::NonExistentPlugin),
            Ok(Some(plugin)) => {
                let previous_versions = plugin.versions().unwrap();
                for previous_version in previous_versions {
                    let parsed_version = parse_semver(&previous_version.version);
                    match parsed_version.cmp(&semversion) {
                        Ordering::Equal => return Err(CreateVersionError::AlreadyExists),
                        Ordering::Greater => return Err(CreateVersionError::LessThanLatestVersion),
                        _ => {}
                    }
                }
                
                let digest = digest_version(&version.version, &plugin.name, &version.tar);
                let version_tar_path = self.version_path(&plugin.name, &version.version);
                std::fs::write(version_tar_path, &version.tar).unwrap();
                db_client
                    .version()
                    .create(
                        version.version,
                        prisma::plugin::name::equals(plugin.name.clone()),
                        false,
                        digest,
                        version.preview,
                        vec![],
                    )
                    .exec()
                    .await
                    .map_err(|_| CreateVersionError::DatabaseError)?;
                Ok(())
            }
            Err(e) => {
                eprintln!("Failed to fetch the plugin from the db: {:#?}", e);
                Err(CreateVersionError::DatabaseError)
            }
        }
    }

    async fn yank_version(
        &mut self,
        plugin_name: String,
        version: String,
    ) -> Result<(), YankVersionError> {
        let db_client = db::connect().await.map_err(|e| {
            eprintln!("Failed to connect to the database: {:#?}", e);
            YankVersionError::DatabaseError
        })?;
        if let Some(v) = db_client
            .version()
            .find_unique(prisma::version::version_plugin_name(
                version.clone(),
                plugin_name.clone(),
            ))
            .exec()
            .await
            .map_err(|_| YankVersionError::DatabaseError)?
        {
            if v.yanked {
                return Err(YankVersionError::NonExistentOrAlreadyYanked);
            }
        } else {
            return Err(YankVersionError::NonExistentOrAlreadyYanked);
        }
        db_client
            .version()
            .update(
                prisma::version::version_plugin_name(version.clone(), plugin_name.clone()),
                vec![prisma::version::yanked::set(true)],
            )
            .exec()
            .await
            .map_err(|_e| YankVersionError::DatabaseError)?;
        Ok(())
    }

    async fn unpublish_plugin(&mut self, plugin_name: String) -> Result<(), UnpublishPluginError> {
        let db_client = db::connect().await.map_err(|e| {
            eprintln!("Failed to connect to the database: {:#?}", e);
            UnpublishPluginError::DatabaseError
        })?;
        let plugin = db_client
            .plugin()
            .find_unique(prisma::plugin::name::equals(plugin_name.clone()))
            .with(prisma::plugin::versions::fetch(vec![]))
            .exec()
            .await
            .unwrap();
        if let Some(plugin) = plugin {
            let versions = plugin.versions().unwrap();
            for version in versions {
                self.remove_version(plugin_name.clone(), version.version.clone())
                    .unwrap();
            }
            db_client.version().delete_many(vec![prisma::version::plugin_name::equals(plugin.name.clone())]).exec().await.unwrap();
            db_client.plugin().delete(prisma::plugin::name::equals(plugin.name.clone())).exec().await.unwrap();
            Ok(())
        } else {
            Err(UnpublishPluginError::NonExistent)
        }
    }
}
fn digest_version(version: &str, plugin_name: &str, tar: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(tar);
    hasher.update(version.as_bytes());
    hasher.update(plugin_name.as_bytes());
    hex::encode(hasher.finalize())
}