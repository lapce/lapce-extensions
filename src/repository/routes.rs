use super::{FileSystemRepository, GetResourceError, NewVoltInfo, PublishError, Repository, UnpublishPluginError};
use crate::db::prisma;
use crate::error::ErrorKind;
use crate::{error::Error, Session};
use rocket::form::Form;
use rocket::http::Status;
use rocket::response::status::Created;
use rocket::{fs::TempFile, serde::json::Json};

#[derive(FromForm)]
pub struct Icon<'r> {
    icon: TempFile<'r>,
}

#[derive(FromForm)]
pub struct Plugin<'r> {
    #[field(validate = len(1..30))]
    display_name: &'r str,
    #[field(validate = len(..500))]
    description: &'r str,
    icon: Option<TempFile<'r>>,
}

fn get_repo() -> impl Repository {
    dotenvy::dotenv().ok();
    match std::env::var("STORAGE") {
        Ok(storage) => match storage.as_str() {
            "filesystem" => FileSystemRepository::default(),
            _ => FileSystemRepository::default(),
        },
        Err(_) => FileSystemRepository::default(),
    }
}
#[patch("/volts/<name>/icon", data = "<icon>")]
pub async fn change_plugin_icon<'a>(
    session: Session<'a>,
    name: String,
    mut icon: Form<Icon<'a>>,
) -> Result<Option<()>, (Status, Json<Error>)> {
    let session = match session.get().await {
        Ok(Some(session)) => session,
        _ => {
            return Err((
                Status::Unauthorized,
                Json(Error {
                    kind: ErrorKind::NotLoggedIn,
                    action: "Send a `token` cookie.".into(),
                    message: "Unauthorized".into(),
                }),
            ))
        }
    };
    let icon = {
        let f = &mut icon.icon;
        let temp_path = std::env::temp_dir().join(names::Generator::default().next().unwrap());
        f.persist_to(temp_path).await.unwrap();
        std::fs::read(f.path().unwrap()).unwrap()
    };
    let mut repo = get_repo();
    let db = match crate::db::connect().await {
        Ok(db) => db,
        Err(err) => return Err((Status::InternalServerError, Json(err))),
    };
    let user = match db
        .user()
        .find_unique(prisma::user::id::equals(session.id.try_into().unwrap()))
        .exec()
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err((
                Status::InternalServerError,
                Json(Error {
                    action: "Try Logging out and logging in".into(),
                    message: "The logged in user doesn't exist?".into(),
                    kind: ErrorKind::ValidationError,
                }),
            ))
        }
        Err(err) => {
            return Err((
                Status::InternalServerError,
                Json(Error {
                    kind: ErrorKind::DatabaseError(err.to_string()),
                    action: "Try again later".into(),
                    message: "Couldn't get the user from the database".into(),
                }),
            ))
        }
    };
    let plugin_id = format!("{}.{}", user.username, name);
    match repo.get_plugin(plugin_id.clone()).await {
        Ok(_) => (),
        Err(GetResourceError::NotFound) => return Ok(None),
        Err(_) => {
            return Err((
                Status::InternalServerError,
                Json(Error {
                    kind: ErrorKind::DatabaseError("".into()),
                    action: "Try again later".into(),
                    message: "Couldn't get the user from the database".into(),
                }),
            ))
        }
    }
    match repo.save_icon(plugin_id.clone(), &icon).await {
        Ok(()) => Ok(Some(())),
        Err(PublishError::InvalidIcon) => Err((
            Status::BadRequest,
            Json(Error {
                kind: ErrorKind::ValidationError,
                action: "Send a image smaller than 2000x2000 and smaller than 200mb".into(),
                message: "The icon is invalid!".into(),
            }),
        )),
        Err(PublishError::IoError) => Err((
            Status::InternalServerError,
            Json(Error {
                kind: ErrorKind::DatabaseError("".into()),
                action: "Try again later".into(),
                message: "Can't save the icon".into(),
            }),
        )),
        _ => unreachable!(),
    }
}

#[get("/volts/<name>")]
pub async fn get_plugin_info<'a>(
    name: String,
) -> Result<Option<Json<prisma::plugin::Data>>, (Status, Json<Error>)> {
    let mut repo = get_repo();
    match repo.get_plugin(name.clone()).await {
        Ok(plugin) => Ok(Some(Json(plugin))),
        Err(GetResourceError::NotFound) => Ok(None),
        Err(_) => Err((
            Status::InternalServerError,
            Json(Error {
                kind: ErrorKind::DatabaseError("".into()),
                action: "Try again later".into(),
                message: "Couldn't get the plugin from the database".into(),
            }),
        )),
    }
}

#[get("/volts/<name>/icon")]
pub async fn get_plugin_icon<'a>(name: String) -> Result<Option<Vec<u8>>, (Status, Json<Error>)> {
    let mut repo = get_repo();
    match repo.get_plugin_icon(name.clone()).await {
        Ok(icon) => Ok(Some(icon)),
        Err(GetResourceError::NotFound) => Ok(None),
        Err(_) => Err((
            Status::InternalServerError,
            Json(Error {
                kind: ErrorKind::DatabaseError("".into()),
                action: "Try again later".into(),
                message: "Couldn't get the user from the database".into(),
            }),
        )),
    }
}

#[post("/volts/<name>", data = "<plugin>")]
pub async fn create_plugin<'a>(
    session: Session<'a>,
    name: String,
    mut plugin: Form<Plugin<'a>>,
) -> Result<Created<Json<prisma::plugin::Data>>, (Status, Json<Error>)> {
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit())
    {
        Err((
            Status::BadRequest,
            Json(Error {
                kind: ErrorKind::ValidationError,
                action:
                    "Use a name that has no spaces and is lowercase. kebab-case is allowed too."
                        .into(),
                message: "The plugin name is invalid!".into(),
            }),
        ))
    } else {
        let icon = {
            let display_name = plugin.display_name.to_string();
            match &mut plugin.icon {
                Some(ref mut f) => Some({
                    let temp_path = std::env::temp_dir().join(format!("{name}-{display_name}"));
                    f.persist_to(temp_path).await.unwrap();
                    std::fs::read(f.path().unwrap()).unwrap()
                }),
                None => None,
            }
        };
        match session.get().await {
            Ok(Some(session)) => {
                let mut repo = get_repo();
                let publish_result = repo
                    .publish(NewVoltInfo {
                        name,
                        description: plugin.description.into(),
                        display_name: plugin.display_name.into(),
                        icon,
                        publisher_id: session.id as i64,
                    })
                    .await;
                match publish_result {
                    Ok(data) => Ok(Created::new("").body(Json(data))),
                    Err(err) => match err {
                        PublishError::AlreadyExists => {
                            Err((Status::Conflict, Json(Error {
                                kind: ErrorKind::ValidationError,
                                action: "Use a different name".into(),
                                message: "The plugin name is already taken!".into()
                            })))
                        },
                        PublishError::DatabaseError => {
                            Err((Status::InternalServerError, Json(Error {
                                kind: ErrorKind::DatabaseError("".into()),
                                action: "Try again later".into(),
                                message: "Failed to write to the database!".into()
                            })))
                        }
                        PublishError::InvalidIcon => {
                            Err((Status::BadRequest, Json(Error {
                                kind: ErrorKind::ValidationError,
                                action: "Send a valid image with resolution less than 2000x2000 and less than 200mb".into(),
                                message: "The icon is invalid!".into()
                            })))
                        }
                        PublishError::IoError => {
                            Err((Status::InternalServerError, Json(Error {
                                kind: ErrorKind::DatabaseError("IoError".into()),
                                action: "Try again later".into(),
                                message: "Failed to save the icon due to a IO error!".into()
                            })))
                        }
                    }
                }
            }
            _ => Err((
                Status::Unauthorized,
                Json(Error {
                    kind: ErrorKind::NotLoggedIn,
                    action: "Send a `token` cookie.".into(),
                    message: "Unauthorized".into(),
                }),
            )),
        }
    }
}

#[delete("/volts/<name>")]
pub async fn delete_plugin(
    session: Session<'_>,
    name: String,
) -> Result<(), (Status, Json<Error>)> {
    let session = match session.get().await {
        Ok(Some(session)) => session,
        _ => return Err((
            Status::Unauthorized,
            Json(Error {
                kind: ErrorKind::NotLoggedIn,
                action: "Send a `token` cookie.".into(),
                message: "Unauthorized".into(),
            }),
        )),
    };
    let db = match crate::db::connect().await {
        Ok(db) => db,
        Err(err) => return Err((Status::InternalServerError, Json(err))),
    };
    let user = match db
        .user()
        .find_unique(prisma::user::id::equals(session.id.try_into().unwrap()))
        .exec()
        .await
    {
        Ok(Some(user)) => user,
        Ok(None) => {
            return Err((
                Status::InternalServerError,
                Json(Error {
                    action: "Try Logging out and logging in".into(),
                    message: "The logged in user doesn't exist?".into(),
                    kind: ErrorKind::ValidationError,
                }),
            ))
        }
        Err(err) => {
            return Err((
                Status::InternalServerError,
                Json(Error {
                    kind: ErrorKind::DatabaseError(err.to_string()),
                    action: "Try again later".into(),
                    message: "Couldn't get the user from the database".into(),
                }),
            ))
        }
    };
    let plugin_id = format!("{}.{}", user.username, name);
    let mut repo = get_repo();
    repo.unpublish_plugin(plugin_id)
        .await.map_err(|err| match err {
            UnpublishPluginError::NonExistent => (
                Status::Conflict,
                Json(Error {
                    kind: ErrorKind::ValidationError,
                    action: "Verify the plugin name".into(),
                    message: "The plugin doesn't exist!".into(),
                }),
            ),
            UnpublishPluginError::DatabaseError => (
                Status::InternalServerError,
                Json(Error {
                    kind: ErrorKind::DatabaseError("".into()),
                    action: "Try again later".into(),
                    message: "Failed to write to the database!".into(),
                }),
            ),
            UnpublishPluginError::IOError => (
                Status::InternalServerError,
                Json(Error {
                    kind: ErrorKind::DatabaseError("IoError".into()),
                    action: "Try again later".into(),
                    message: "An unexpected IO error happened!".into(),
                }),
            ),
        })
}
