use rocket::http::{Cookie, CookieJar, Status};
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket_oauth2::{OAuth2, TokenResponse};
use octorust::Client;
use octorust::auth::Credentials;
use rocket::response::status;
use rocket_session_store::Session;
use crate::db::establish_connection;
use crate::error::*;
use crate::user::User;
pub struct GitHub;

#[get("/login/github")]
pub fn github_login(oauth2: OAuth2<GitHub>, cookies: &CookieJar<'_>) -> Redirect {
    oauth2.get_redirect(cookies, &["user:read"]).unwrap()
}

#[get("/auth/github")]
pub async fn github_callback(token: TokenResponse<GitHub>, session: Session<'_, i64>) -> Result<Redirect, status::Custom<Json<Error>>>
{
    let gh_token =token.access_token().to_string();
    let github = Client::new("LapceExtensions", Credentials::Token(gh_token.into()));
    match github {
        Ok(github) => {
            let users = github.users();
            let user = users.get_authenticated().await.unwrap();
            let user = user.public_user().unwrap();
            if let Err(err) = session.set(user.id).await {
                Err(status::Custom(Status::InternalServerError, Json(Error {
                    kind: ErrorKind::DatabaseError(err.to_string()),
                    action: "Try again".into(),
                    message: "Can't set session on redis db".into()
                })))
            } else {
                let user = User {
                    id: user.id,
                    name: user.name.clone(),
                    username: user.login.clone(),
                    avatar_url: user.avatar_url.clone()
                };
                use diesel::prelude::*;
                use crate::schema::users::dsl::*;
                let connection = establish_connection();
                match connection {
                    Ok(mut connection) => {
                        diesel::insert_into(users)
                            .values(&user)
                            .on_conflict(id)
                            .do_update()
                            .set(&user)
                            .execute(&mut connection).unwrap();
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