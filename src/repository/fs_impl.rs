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
    async fn get_plugin_version_wasm(
        &mut self,
        name: String,
        version: String,
    ) -> Result<Vec<u8>, GetResourceError> {
        {
            let version = self
                .get_plugin_version(name.clone(), version.clone())
                .await?;
            if version.yanked {
                return Err(GetResourceError::NotFound);
            }
        }
        let mut path = self.base_path();
        path.push("versions/");
        path.push(format!("{}-{}", &name, &version));
        path.push("plugin.wasm");
        if path.try_exists().map_err(|_| GetResourceError::IoError)? {
            std::fs::read(path).map_err(|_| GetResourceError::IoError)
        } else {
            Err(GetResourceError::NotFound)
        }
    }
    async fn get_plugin_version_themes(
        &mut self,
        name: String,
        version: String,
    ) -> Result<Vec<String>, GetResourceError> {
        let version = self
            .get_plugin_version(name.clone(), version.clone())
            .await?;
        if version.yanked {
            Err(GetResourceError::NotFound)
        } else {
            let mut base_dir = self.base_path();
            base_dir.push("versions/");
            base_dir.push(format!("{}-{}", &name, &version.version));
            base_dir.push("themes");
            let mut themes = vec![];
            for files in std::fs::read_dir(base_dir).unwrap() {
                let files = files.unwrap();
                if files.file_type().unwrap().is_file() {
                    let path = files.path();
                    themes.push(std::fs::read_to_string(path).unwrap());
                }
            }
            Ok(themes)
        }
    }
    async fn get_plugin_icon(&mut self, name: String) -> Result<Vec<u8>, GetResourceError> {
        let mut path = self.base_path();
        path.push("icons/");
        path.push(&name);
        if path.try_exists().map_err(|_| GetResourceError::IoError)? {
            std::fs::read(path).map_err(|_| GetResourceError::IoError)
        } else {
            Err(GetResourceError::NotFound)
        }
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
                let mut themes_folder = base_dir.clone();
                themes_folder.push("themes");
                std::fs::create_dir_all(themes_folder.clone()).unwrap();
                for (i, t) in version.themes.clone().iter().enumerate() {
                    let mut file = themes_folder.clone();
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
        if let Some(v) = db_client
            .version()
            .find_unique(prisma::version::version_plugin_name(
                plugin_name.clone(),
                version.clone(),
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
                prisma::version::version_plugin_name(plugin_name.clone(), version.clone()),
                vec![],
            )
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
#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::FileSystemRepository;
    use crate::db::prisma;
    use crate::repository::NewVoltInfo;
    use crate::repository::PublishError;
    use crate::repository::Repository;
    use rocket::tokio;
    #[tokio::test]
    async fn publish_plugin() {
        dotenvy::dotenv().unwrap();
        let db = prisma::new_client().await.unwrap();
        db.user()
            .create(
                /* ID: */ 1,
                /* Display Name: */ "Tests".into(),
                /* Login name: */ "tests".into(),
                /* Avatar URL: */ "https://example.com".into(),
                vec![],
            )
            .exec()
            .await
            .unwrap();
        let mut repo = FileSystemRepository::default();
        // Icons bigger than 2000X2000 should be considered invalid
        // Money doesn't grow in trees!
        let invalid_icon = image::RgbaImage::new(4000, 4000);
        let mut invalid_icon_bytes = vec![];
        invalid_icon
            .write_to(
                &mut Cursor::new(&mut invalid_icon_bytes),
                image::ImageOutputFormat::Png,
            )
            .unwrap();
        assert_eq!(
            repo.publish(NewVoltInfo {
                name: "my_plugin".into(),
                display_name: "My Test plugin".into(),
                description: "Dummy plugin".into(),
                author: "tests".into(),
                publisher_id: 1,
                icon: Some(invalid_icon_bytes),
            })
            .await
            .unwrap_err(),
            PublishError::InvalidIcon
        );
        // The icon is valid, so the plugin should be published successfully
        let valid_icon = image::RgbaImage::new(100, 100);
        let mut valid_icon_bytes: Vec<u8> = Vec::new();
        valid_icon
            .write_to(
                &mut Cursor::new(&mut valid_icon_bytes),
                image::ImageOutputFormat::Png,
            )
            .unwrap();

        repo.publish(NewVoltInfo {
            name: "my_plugin".into(),
            display_name: "My Test plugin".into(),
            description: "Dummy plugin".into(),
            author: "tests".into(),
            publisher_id: 1,
            icon: Some(valid_icon_bytes),
        })
        .await
        .unwrap();
        // The plugin should be in the database here
        let new_plugin = db
            .plugin()
            .find_unique(prisma::plugin::name::equals("my_plugin".into()))
            .exec()
            .await
            .unwrap()
            .unwrap();
        // Make some sanity checks before assuming the code is OK
        assert_eq!(new_plugin.name, "my_plugin".to_string());
        assert_eq!(new_plugin.display_name, "My test plugin".to_string());
        assert_eq!(new_plugin.description, "Dummy plugin");
        assert_eq!(new_plugin.author, "tests");
        assert_eq!(new_plugin.publisher_id, 1);
    }
}
