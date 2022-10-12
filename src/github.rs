use rocket::http::{CookieJar, Status};
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket_oauth2::{OAuth2, TokenResponse};
use octorust::Client;
use octorust::auth::Credentials;
use rocket::response::status;
use crate::{Session, SessionInfo};
use crate::db::*;
use crate::error::*;
pub struct GitHub;

#[get("/login/github")]
pub fn github_login(oauth2: OAuth2<GitHub>, cookies: &CookieJar<'_>) -> Redirect {
    oauth2.get_redirect(cookies, &["read:user"]).unwrap()
}

#[get("/auth/github")]
pub async fn github_callback(token: TokenResponse<GitHub>, session: Session<'_>) -> Result<Redirect, status::Custom<Json<Error>>>
{
    let gh_token =token.access_token().to_string();
    let github = Client::new("LapceExtensions", Credentials::Token(gh_token.clone().into()));
    match github {
        Ok(github) => {
            let users = github.users();
            let user = users.get_authenticated().await.unwrap();
            let user = user.public_user().unwrap();
            if let Err(err) = session.set(SessionInfo {
                gh_token,
                id: user.id as u64
            }).await {
                Err(status::Custom(Status::InternalServerError, Json(Error {
                    kind: ErrorKind::DatabaseError(err.to_string()),
                    action: "Try again".into(),
                    message: "Can't set session on redis db".into()
                })))
            } else {
                let client = establish_connection().await;
                match client {
                    Ok(client) => {
                        client.user().upsert(
                            prisma::user::id::equals(user.id),
                            prisma::user::create(
                                user.id,
                                user.login.clone(),
                                user.name.clone(),
                                user.avatar_url.clone(),
                                vec![]
                            ), 
                            vec![
                                prisma::user::name::set(user.login.clone()),
                                prisma::user::username::set(user.name.clone()),
                                prisma::user::avatar_url::set(user.avatar_url.clone()),
                            ]
                        ).exec().await.unwrap();
                        Ok(Redirect::to("/"))
                    }
                    Err(err) => {
                        Err(status::Custom(Status::InternalServerError, Json(err)))
                    }
                }
            }
        }
        Err(_) => Err(status::Custom(Status::InternalServerError, Json(Error {
            kind: ErrorKind::GithubApiError,
            action: "Try again".into(),
            message: "Can't fetch from github api".into()
        })))
    }
}
