use std::{path::PathBuf};

use diesel::{Insertable, Queryable, QueryDsl};
use octorust::{Client, auth::Credentials, types::FullRepository};
use rocket::{tokio::{fs::File, io::{AsyncWriteExt, AsyncReadExt}}, serde::{json::Json, Deserialize, Serialize}, http::{ContentType, Status}};
use crate::{Session, user::User, db::establish_connection, schema::plugins, error::ErrorKind};
use crate::error::Error;

#[derive(Queryable)]
pub struct PluginRepo {
    id: u32,
    user_id: u64,
    owner: String,
    repo: String
}
#[derive(Insertable)]
#[diesel(table_name = crate::schema::plugins)]
pub struct NewPluginRepo {
    user_id: i64,
    owner: String,
    repo: String
}
#[async_trait]
pub trait PluginWasmStore {
    async fn store<'f>(&self, id: u32, bytes: Vec<u8>) -> std::io::Result<()>;
    async fn get<'f>(&self, id: u32) -> std::io::Result<Vec<u8>>;
    async fn exists<'f>(&self, id: u32) -> std::io::Result<bool>;
}
pub struct FileSystemPluginWasmStore {
    directory: PathBuf
}
impl Default for FileSystemPluginWasmStore {
    fn default() -> Self {
        Self::new(PathBuf::from("./fs-registry/"))
    }
}
impl FileSystemPluginWasmStore {
    pub fn new(directory: PathBuf) -> Self {
        Self { directory }
    }
    fn get_path(&self, id: u32) -> PathBuf {
        let mut path = self.directory.join(id.to_string());
        path.set_extension("wasm");
        path
    }
}
#[async_trait]
impl PluginWasmStore for FileSystemPluginWasmStore {
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
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "rocket::serde")]
pub struct PluginInfo {
    pub name: String,
    pub display_name: String,
    pub repository: String,
    pub author: String,
    pub publisher_id: u64,
    pub publisher_name: String,
    pub wasm_file_name: Option<String>,
    pub description: String,
    pub themes: Option<Vec<String>>,
    pub versions: Vec<String>,
    pub readme: Option<String>
}
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "rocket::serde")]
pub struct Volt {
    name: String,
    version: String,
    author: String,
    #[serde(rename = "display-name")]
    display_name: String,
    description: String,
    wasm: Option<String>,
    themes: Option<Vec<String>>,
}
pub async fn get_plugin_info_from_repo(user: User, owner: String, repo: String, tag: String, client: Client) -> PluginInfo {
    let volt_text = reqwest::get(format!("https://raw.githubusercontent.com/{owner}/{repo}/{tag}/volt.toml")).await.unwrap().text().await.unwrap();
    let volt = toml::from_str::<Volt>(&volt_text).unwrap();
    let readme = reqwest::get(format!("https://raw.githubusercontent.com/{owner}/{repo}/{tag}/README.md")).await.ok().map(|res| async {res.text().await.unwrap()});
    PluginInfo {
        author: volt.author,
        description: volt.description,
        display_name: volt.display_name,
        name: volt.name,
        publisher_id: user.id as u64,
        publisher_name: user.name,
        readme: if let Some(readme) = readme {Some(readme.await)} else {None},
        repository: format!("https://github.com/{owner}/{repo}.git/"),
        themes: volt.themes,
        versions: client.repos().list_all_releases(&owner, &repo).await.unwrap().iter().map(|x| x.tag_name.clone()).collect(),
        wasm_file_name: volt.wasm,
    }
}
/// Registers a github repository as a plugin and returns PluginInfo from the latest tag
#[post("/plugin/<owner>/<repo>")]
pub async fn publish_plugin(owner: String, repo: String, session: Session<'_>) -> Result<Option<Json<PluginInfo>>, (Status, Json<Error>)> {
    let session = session.get().await.unwrap();
    match session {
        None => {
            Err((Status::Unauthorized, Json(Error {
                kind: crate::error::ErrorKind::NotLoggedIn,
                action: "Send a `token` cookie".into(),
                message: "You're not logged in because the token cookie is missing.".into()
            })))
        }
        Some(session) => {
            let github = Client::new("LapceExtensions", Credentials::Token(session.gh_token.clone().into())).unwrap();
            let gh_repo = match github.repos().get(&owner, &repo).await {
                Ok(repo) => repo,
                Err(_) => return Ok(None)
            };
            let permissions = gh_repo.permissions.unwrap();
            if permissions.admin && permissions.push && permissions.pull {

                let connection = establish_connection();
                let mut connection = match connection {
                    Ok(connection) => connection,
                    Err(err) => {
                        return Err((Status::InternalServerError, Json(err)))
                    }
                };
                use diesel::prelude::*;
                let user: User = {
                    use crate::schema::users::dsl::*;
                    users.find(session.id as i64).first(&mut connection).unwrap()
                };
                use crate::schema::plugins::dsl::plugins;
                {
                    use crate::schema::plugins::dsl::{owner as owner_col, repo as repo_col};
                    use diesel::prelude::*;
                    if let Ok(count) = plugins.filter(owner_col.eq(owner.clone())).filter(repo_col.eq(repo.clone())).count().get_result::<i64>(&mut connection){
                        if count > 0 {
                            return Err((Status::Forbidden, Json(Error {
                                kind: ErrorKind::ValidationError,
                                message: "That plugin is already published".into(),
                                action: "Try a different repository".into()
                            })))
                        }
                    }
                }
                let plugin_info = get_plugin_info_from_repo(user.clone(), owner.clone(), repo.clone(), github.repos().get_latest_release(&owner, &repo).await.unwrap().tag_name, github).await;
                diesel::insert_into(plugins)
                    .values(NewPluginRepo {
                        owner,
                        repo,
                        user_id: user.id
                    })
                    .execute(&mut connection)
                    .unwrap();
                Ok(Some(Json(plugin_info)))
            } else {
                Err((Status::Unauthorized, Json(Error {
                    kind: crate::error::ErrorKind::ValidationError,
                    action: "Verify the repo you tried to publish".into(),
                    message: "You can't publish a plugin from a repository that you don't own".into()
                })))
            }
        }
    }
}
/// 
#[get("/plugin/<owner>/<repo>")]
pub async fn get_plugin_info(owner: String, repo: String) -> Option<Json<PluginInfo>>{
    None
}

#[get("/plugin/<owner>/<repo>/<tag>/zip")]
pub async fn get_plugin_zip(owner: String, repo: String, tag: String) -> Option<(ContentType, Vec<u8>)> {
    None
}