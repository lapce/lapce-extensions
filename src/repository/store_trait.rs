use std::io::Cursor;

use rocket::serde::*;

use crate::db::{self, prisma};
pub type Blob = Vec<u8>;
/// Contains the necessary information to create a new plugin
#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct NewVoltInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub author: String,
    pub publisher_id: i64,
    pub icon: Option<Blob>,
}
/// This struct is used to create a new version of a plugin
#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct NewPluginVersion {
    /// This contains all the bytes of the wasm file containing the code
    pub wasm_file: Option<Blob>,
    /// The new version, must be greater than existing versions
    pub version: String,
    /// Contains all the theme toml files, they're simple text, so we represent as a array of strings
    pub themes: Vec<String>,
    /// Tells if this version a pre-release or not
    pub preview: bool,
}
/// Represents a error that might happen when publishing a plugin
pub enum PublishError {
    /// Couldn't store the plugin in the database
    DatabaseError,
    /// A plugin with the same name already exists
    AlreadyExists,
    InvalidIcon,
    IoError,
}
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
/// Represents a error that might happen when yanking a version
pub enum YankVersionError {
    /// The version doesn't exist or was already yanked previously
    NonExistentOrAlreadyYanked,
    /// Couldn't store the plugin in the database
    DatabaseError,
}
/// Represents a error that might happen when unpublishing a plugin
pub enum UnpublishPluginError {
    /// There was an error while removing the plugin
    IOError,
    /// The plugin doesn't exist
    NonExistent,
    /// Couldn't store the plugin in the database
    DatabaseError,
}
pub enum IconValidationError {
    TooBig { width: u32, height: u32 },
    NotAnImage,
}
pub fn validate_icon(icon: Blob) -> Option<IconValidationError> {
    let parsed_image = image::io::Reader::new(Cursor::new(icon)).with_guessed_format();
    let parsed_image = match parsed_image {
        Ok(i) => i.decode(),
        Err(_) => {
            return Some(IconValidationError::NotAnImage);
        }
    };
    let parsed_image = match parsed_image {
        Ok(i) => i,
        Err(_) => {
            return Some(IconValidationError::NotAnImage);
        }
    };
    if parsed_image.width() > 2000 || parsed_image.height() > 2000 {
        Some(IconValidationError::TooBig {
            width: parsed_image.width(),
            height: parsed_image.height(),
        })
    } else {
        None
    }
}
pub enum GetResourceError {
    NotFound,
    DatabaseError(prisma_client_rust::QueryError),
    DatabaseConnectionError(prisma_client_rust::NewClientError),
    IoError
}
#[async_trait]
pub trait Repository {
    async fn get_plugin_version(&mut self, name: String, version: String) -> Result<prisma::version::Data, GetResourceError> {
        let db_client = prisma::new_client().await.map_err(|e| {
            eprintln!("Failed to connect to the database: {:#?}", &e);
            GetResourceError::DatabaseConnectionError(e)
        })?;
        let version_result = db_client.version().find_unique(prisma::version::version_plugin_name(version, name)).exec().await;
        match version_result {
            Ok(Some(version)) => Ok(version),
            Ok(None) => Err(GetResourceError::NotFound),
            Err(e) => Err(GetResourceError::DatabaseError(e))
        }
    }
    async fn get_plugin(&mut self, name: String) -> Result<prisma::plugin::Data, GetResourceError> {
        let db_client = prisma::new_client().await.map_err(|e| {
            eprintln!("Failed to connect to the database: {:#?}", &e);
            GetResourceError::DatabaseConnectionError(e)
        })?;
        let plugin_result = db_client.plugin().find_unique(prisma::plugin::name::equals(name)).exec().await;
        match plugin_result {
            Ok(Some(plugin)) => Ok(plugin),
            Ok(None) => Err(GetResourceError::NotFound),
            Err(e) => Err(GetResourceError::DatabaseError(e))
        }
    }
    async fn get_plugin_version_wasm(&mut self, name: String, version: String) -> Result<Vec<u8>, GetResourceError>;
    async fn get_plugin_version_themes(&mut self, name: String, version: String) -> Result<Vec<String>, GetResourceError>;
    async fn get_plugin_icon(&mut self, name: String) -> Result<Vec<u8>, GetResourceError>;
    /// Creates a new plugin, plugins are containers that store versions
    /// and versions store the actual plugin data, like the code and themes
    async fn publish(&mut self, volt_info: NewVoltInfo) -> Result<(), PublishError> {
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
        db_client
            .plugin()
            .create(
                volt_info.name.clone(),
                volt_info.description.clone(),
                volt_info.display_name.clone(),
                volt_info.author.clone(),
                prisma::user::id::equals(volt_info.publisher_id),
                vec![],
            )
            .exec()
            .await
            .map_err(|e| {
                eprintln!("Failed to create a new plugin: {:#?}", e);
                PublishError::DatabaseError
            })?;
        if let Some(icon) = volt_info.icon {
            self.save_icon(volt_info.name.clone(), icon).await
        } else {
            Ok(())
        }
    }
    /// Saves the plugin icon
    async fn save_icon(
        &mut self,
        plugin_name: String,
        icon: super::Blob,
    ) -> Result<(), PublishError>;
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
