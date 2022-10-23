use crate::db::{self, prisma};
use rocket::serde::*;

pub type Blob = Vec<u8>;
/// Contains the necessary information to create a new plugin
#[derive(Deserialize, Serialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct NewVoltInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub publisher_id: i64,
    pub icon: Option<Blob>,
}
/// This struct is used to create a new version of a plugin
#[derive(Deserialize, Serialize, Debug)]
#[serde(crate = "rocket::serde")]
pub struct NewPluginVersion {
    /// The new version, must be greater than existing versions
    pub version: String,
    /// tar gz file of this version
    pub tar: Blob,

    /// Tells if this version a pre-release or not
    pub preview: bool,
}
/// Represents a error that might happen when publishing a plugin
#[derive(Debug, PartialEq, Eq)]
pub enum PublishError {
    /// Couldn't store the plugin in the database
    DatabaseError,
    /// A plugin with the same name already exists
    AlreadyExists,
    InvalidIcon,
    IoError,
}
#[derive(Debug, PartialEq, Eq)]
/// Represents a error that might happen when creating a version
pub enum CreateVersionError {
    /// There was an error while trying to write the files
    IOError,
    /// The version already exists
    AlreadyExists,
    /// There is already a greater version
    LessThanLatestVersion,
    /// The plugin doesn't exist
    NonExistentPlugin,
    /// Couldn't store the plugin in the database
    DatabaseError,
    /// The version is not a valid semver (see https://semver.org)
    InvalidSemVer,
}
#[derive(Debug, PartialEq, Eq)]
/// Represents a error that might happen when yanking a version
pub enum YankVersionError {
    /// The version doesn't exist or was already yanked previously
    NonExistentOrAlreadyYanked,
    /// Couldn't store the plugin in the database
    DatabaseError,
}
#[derive(Debug, PartialEq, Eq)]
/// Represents a error that might happen when unpublishing a plugin
pub enum UnpublishPluginError {
    /// There was an error while removing the plugin
    IOError,
    /// The plugin doesn't exist
    NonExistent,
    /// Couldn't store the plugin in the database
    DatabaseError,
}
#[derive(Debug, PartialEq, Eq)]
pub enum IconValidationError {
    TooBig { width: usize, height: usize },
    NotAnImage,
}
pub fn validate_icon(icon: &[u8]) -> Option<IconValidationError> {
    let (width, height) = match imagesize::blob_size(icon) {
        Ok(dim) => (dim.width, dim.height),
        Err(_) => return Some(IconValidationError::NotAnImage),
    };
    if width > 500 || height > 500 || icon.len() > /* 20 MB*/ 2 * (10usize.pow(7)) {
        Some(IconValidationError::TooBig { width, height })
    } else {
        None
    }
}
#[derive(Debug)]
pub enum GetResourceError {
    NotFound,
    DatabaseError,
    DatabaseConnectionError,
    IoError,
}
#[async_trait]
pub trait Repository {
    async fn get_plugin_version_tar(
        &mut self,
        name: String,
        version: String,
    ) -> Result<Vec<u8>, GetResourceError>;
    async fn get_plugin_version(
        &mut self,
        name: String,
        version: String,
    ) -> Result<prisma::version::Data, GetResourceError> {
        let db_client = prisma::new_client().await.map_err(|e| {
            eprintln!("Failed to connect to the database: {:#?}", &e);
            GetResourceError::DatabaseConnectionError
        })?;
        let version_result = db_client
            .version()
            .find_unique(prisma::version::version_plugin_name(version, name))
            .exec()
            .await;
        match version_result {
            Ok(Some(version)) => {
                if version.yanked {
                    Err(GetResourceError::NotFound)
                } else {
                    Ok(version)
                }
            },
            Ok(None) => Err(GetResourceError::NotFound),
            Err(_) => Err(GetResourceError::DatabaseError),
        }
    }
    async fn get_plugin(&mut self, name: String) -> Result<prisma::plugin::Data, GetResourceError> {
        let db_client = prisma::new_client().await.map_err(|e| {
            eprintln!("Failed to connect to the database: {:#?}", &e);
            GetResourceError::DatabaseConnectionError
        })?;
        let plugin_result = db_client
            .plugin()
            .find_unique(prisma::plugin::name::equals(name))
            .exec()
            .await;
        match plugin_result {
            Ok(Some(plugin)) => Ok(plugin),
            Ok(None) => Err(GetResourceError::NotFound),
            Err(_) => Err(GetResourceError::DatabaseError),
        }
    }
    async fn get_plugin_icon(&mut self, name: String) -> Result<Vec<u8>, GetResourceError>;
    /// Creates a new plugin, plugins are containers that store versions
    /// and versions store the actual plugin data, like the code and themes
    async fn publish(
        &mut self,
        volt_info: NewVoltInfo,
    ) -> Result<prisma::plugin::Data, PublishError> {
        let db_client = db::connect().await.map_err(|e| {
            eprintln!("Failed to connect to the database: {:#?}", e);
            PublishError::DatabaseError
        })?;
        let plugin = db_client
            .plugin()
            .find_unique(prisma::plugin::name::equals(volt_info.name.clone()))
            .exec()
            .await
            .map_err(|e| {
                eprintln!("Failed to fetch the plugin from the db: {:#?}", e);
                PublishError::DatabaseError
            })?;
        if plugin.is_some() {
            println!("Failed to create new plugin: already exists");
            return Err(PublishError::AlreadyExists);
        }
        // Cloning is not cheap, so we get a reference instead
        if let Some(icon) = &volt_info.icon {
            self.save_icon(volt_info.name.clone(), icon).await?;
        }
        let publisher = db_client.user().find_unique(prisma::user::id::equals(volt_info.publisher_id)).exec().await.unwrap().unwrap();
        db_client
            .plugin()
            .create(
                format!("{}.{}", publisher.username, volt_info.name),
                volt_info.description.clone(),
                volt_info.display_name.clone(),
                prisma::user::id::equals(volt_info.publisher_id),
                vec![],
            )
            .exec()
            .await
            .map_err(|e| {
                eprintln!("Failed to create a new plugin: {:#?}", e);
                PublishError::DatabaseError
            })
    }
    /// Saves the plugin icon
    async fn save_icon(&mut self, plugin_name: String, icon: &[u8]) -> Result<(), PublishError>;
    /// Creates a new version on a existing plugin
    async fn create_version(
        &mut self,
        plugin_name: String,
        version: NewPluginVersion,
    ) -> Result<(), CreateVersionError>;
    /// Yanks a version, this makes the version undownloadable and will make lapce prompt to update
    /// Yanking a version is saying that the version is broken, and must be updated
    async fn yank_version(
        &mut self,
        plugin_name: String,
        version: String,
    ) -> Result<(), YankVersionError>;
    /// Unpublishes a plugin from the repository
    async fn unpublish_plugin(&mut self, plugin_name: String) -> Result<(), UnpublishPluginError>;
}
