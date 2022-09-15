use std::{path::PathBuf, default};

use diesel::{Insertable, Queryable};
use rocket::{form::Form, fs::TempFile, tokio::{fs::File, io::{AsyncWriteExt, AsyncReadExt}}};
use rocket_session_store::Session;

#[derive(Queryable)]
pub struct Plugin {
    id: u32,
    user_id: u64,
    name: String,
    description: String,
    version: String,
    display_name: String,
    repository: String
}
#[derive(Insertable)]
#[diesel(table_name = crate::schema::plugins)]
pub struct NewPlugin {
    name: String,
    user_id: i64,
    description: String,
    version: String,
    display_name: String,
    repository: String
}
#[derive(FromForm)]
pub struct PluginForm<'f> {
    file: TempFile<'f>
}
#[async_trait]
pub trait PluginStore {
    async fn store<'f>(&self, id: u32, bytes: Vec<u8>) -> std::io::Result<()>;
    async fn get<'f>(&self, id: u32) -> std::io::Result<Vec<u8>>;
    async fn exists<'f>(&self, id: u32) -> std::io::Result<bool>;
}
pub struct FileSystemPluginStore {
    directory: PathBuf
}
impl Default for FileSystemPluginStore {
    fn default() -> Self {
        Self::new(PathBuf::from("./fs-registry/"))
    }
}
impl FileSystemPluginStore {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }
    fn get_path(&self, id: u32) -> PathBuf {
        let mut path = self.directory.join(id.to_string());
        path.set_extension("zip");
        path
    }
}
#[async_trait]
impl PluginStore for FileSystemPluginStore {
    async fn get<'f>(&self, id: u32) -> std::io::Result<Vec<u8>> {
        let path = self.get_path(id);
        let mut file = File::open(path).await?;
        let mut buf = Vec::new();
        buf.reserve_exact(file.metadata().await?.len() as usize);
        file.read_to_end(&mut buf).await?;
        Ok(buf)
    }
    async fn store<'f>(&self, id: u32, bytes: Vec<u8>) -> std::io::Result<()> {
        let path = self.get_path(id);
        let mut file = File::create(path).await?;
        file.write(&bytes).await?;
        Ok(())
    }
    async fn exists<'f>(&self, id: u32) -> std::io::Result<bool> {
        let path = self.get_path(id);
        path.try_exists()
    }
}
#[post("/upload", data = "<plugin>")]
pub fn upload_plugin(plugin: Form<PluginForm<'_>>, session: Session<'_, i64>){
    
}