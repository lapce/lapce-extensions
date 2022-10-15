use rocket::serde::*;
type Blob = Vec<u8>;
/// Contains the necessary information to create a new plugin
#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct NewVoltInfo {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub author: String,
    pub publisher: String,
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
}
/// Represents a error that might happen when publishing a plugin
pub enum PublishError {
    /// Couldn't store the plugin in the database
    DatabaseError,
    /// A plugin with the same name already exists
    AlreadyExists,
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
}
/// Represents a error that might happen when yanking a version
pub enum YankVersionError {
    /// There was an error while trying to set as yanked
    IOError,
    /// The version doesn't exist or was already yanked previously
    NonExistentOrAlreadyYanked,
    /// The plugin doesn't exist
    NonExistentPlugin,
}
/// Represents a error that might happen when unpublishing a plugin
pub enum UnpublishPluginError {
    /// There was an error while removing the plugin
    IOError,
    /// The plugin doesn't exist
    NonExistent,
}
pub trait Repository {
    /// Creates a new plugin, plugins are containers that store versions
    /// and versions store the actual plugin data, like the code and themes
    fn publish(&mut self, volt_info: NewVoltInfo);
    /// Creates a new version on a existing plugin
    fn create_version(
        &mut self,
        plugin_name: String,
        version: NewPluginVersion,
    ) -> Result<(), CreateVersionError>;
    /// Yanks a version, this makes the version undownloadable and will make lapce prompt to update
    /// Yanking a version is saying that the version is broken, and must be updated
    fn yank_version(
        &mut self,
        plugin_name: String,
        version: String,
    ) -> Result<(), YankVersionError>;
    /// Unpublishes a plugin from the repository
    fn unpublish_plugin(&mut self, plugin_name: String) -> Result<(), UnpublishPluginError>;
}
