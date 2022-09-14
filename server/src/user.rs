
use octorust::Client;
use octorust::auth::Credentials;
use rocket::http::CookieJar;
use rocket::response::status::Unauthorized;
use rocket::serde::json::Json;
use rocket::serde::{Serialize, Deserialize};
use crate::error::*;

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct User {
    name: String,
    username: String,
    id: i64,
    avatar_url: String
}
#[get("/user")]
pub async fn get_user(cookies: &CookieJar<'_>) -> Result<Json<User>, Unauthorized<Json<Error>>>{
    match cookies.get_private("token") {
        Some(token) => {
            let github = Client::new(
              String::from("LapceExtensions"),
              Credentials::Token(
                String::from(token.value())
              ),
            );
            match github {
                Ok(github) => {
                    let users = github.users();
                    let user = users.get_authenticated().await.unwrap();
                    let user = user.public_user().unwrap();
                    Ok(Json(
                        User {
                            id: user.id,
                            username: user.login.clone(),
                            avatar_url: user.avatar_url.clone(),
                            name: user.name.clone()
                        }
                    ))
                }
                Err(_) => Err(Unauthorized(Some(Json(Error {
                    kind: ErrorKind::GithubApiError,
                    action: "Verify your token.".into(),
                    message: "Can't connect to github api".into()
                }))))
            }
        }
        None => {
            Err(Unauthorized(Some(Json(Error {
                kind: ErrorKind::NotLoggedIn,
                action: "Send a `token` cookie.".into(),
                message: "Unauthorized".into()
            }))))
        }
    }
    
    
}