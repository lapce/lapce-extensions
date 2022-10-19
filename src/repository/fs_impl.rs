use crate::db::{self, prisma};

use super::*;
use lazy_static::*;
use std::{path::PathBuf, str::FromStr};
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
    pub fn remove_version(&self, plugin_name: String, version: String) -> std::io::Result<()> {
        let mut path = self.base_path();
        path.push("versions/");
        path.push(format!("{}-{}", &plugin_name, &version));
        std::fs::remove_dir_all(path)?;
        Ok(())
    }
}
#[async_trait]
impl Repository for FileSystemRepository {
    async fn get_plugin(&mut self, name: String) -> Result<prisma::plugin::Data, GetResourceError>{
        todo!("Get Plugin")
    }
    async fn get_plugin_version(&mut self, name: String, version: String) -> Result<prisma::version::Data, GetResourceError> {
        todo!("Get version")
    }
    async fn get_plugin_version_wasm(&mut self, name: String, version: String) -> Result<Vec<u8>, GetResourceError> {
        todo!("Get version wasm file")
    }
    async fn get_plugin_version_themes(&mut self, name: String, version: String) -> Result<Vec<String>, GetResourceError> {
        todo!("Get plugin version themes")
    }
    async fn get_plugin_icon(&mut self, name: String) -> Result<Vec<u8>, GetResourceError> {
        todo!("Get plugin icon")
    }
    async fn save_icon(
        &mut self,
        plugin_name: String,
        icon: super::Blob,
    ) -> Result<(), PublishError> {
        if validate_icon(icon.clone()).is_some() {
            return Err(PublishError::InvalidIcon);
        }
        let mut icon_path = self.base_path();
        icon_path.push("icons");
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
            .exec()
            .await
        {
            Ok(None) => Err(CreateVersionError::NonExistentPlugin),
            Ok(Some(plugin)) => {
                let previous_versions = plugin.versions().unwrap();
                for previous_version in previous_versions {
                    let parsed_version = parse_semver(&previous_version.version);
                    if parsed_version > semversion {
                        return Err(CreateVersionError::LessThanLatestVersion);
                    }
                }
                use sha1::{Digest, Sha1};
                let mut hasher = Sha1::new();
                if let Some(wasm_file) = &version.wasm_file {
                    hasher.update(wasm_file);
                }

                for theme in &version.themes {
                    hasher.update(theme.as_bytes());
                }
                hasher.update(version.version.as_bytes());
                hasher.update(plugin_name.as_bytes());
                let digest = hex::encode(hasher.finalize());
                let mut base_dir = self.base_path();
                base_dir.push("versions/");
                base_dir.push(format!("{}-{}", &plugin.name, &version.version));
                std::fs::create_dir_all(base_dir.clone()).unwrap();
                if let Some(wasm_file) = &version.wasm_file {
                    let mut file = base_dir.clone();
                    file.push("plugin.wasm");
                    std::fs::write(file, wasm_file.clone()).unwrap();
                }
                for (i, t) in version.themes.clone().iter().enumerate() {
                    let mut file = base_dir.clone();
                    file.push("themes");
                    file.push(format!("{}.toml", i));
                    std::fs::write(file, t).unwrap();
                }
                db_client.version().create(
                    version.version,
                    prisma::plugin::name::equals(plugin.name.clone()),
                    false,
                    digest,
                    version.preview,
                    vec![],
                );
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
        let where_param = prisma::version::UniqueWhereParam::VersionPluginNameEquals(
            plugin_name.clone(),
            version.clone(),
        );
        if let Some(v) = db_client
            .version()
            .find_unique(where_param.clone())
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
            .update(where_param.clone(), vec![])
            .exec()
            .await
            .map_err(|_| YankVersionError::NonExistentOrAlreadyYanked)?;
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
            .exec()
            .await
            .unwrap();
        if let Some(plugin) = plugin {
            let versions = plugin.versions().unwrap();
            for version in versions {
                self.remove_version(plugin_name.clone(), version.version.clone())
                    .unwrap();
            }
            Ok(())
        } else {
            Err(UnpublishPluginError::NonExistent)
        }
    }
}
