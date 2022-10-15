use crate::db::*;
use crate::error::*;
use crate::{Session, SessionInfo};
use octorust::auth::Credentials;
use octorust::Client;
use rocket::http::{CookieJar, Status};
use rocket::response::status;
use rocket::response::Redirect;
use rocket::serde::json::Json;
use rocket_oauth2::{OAuth2, TokenResponse};
pub struct GitHub;

#[get("/login/github")]
pub fn login(oauth2: OAuth2<GitHub>, cookies: &CookieJar<'_>) -> Redirect {
    oauth2.get_redirect(cookies, &["read:user"]).unwrap()
}
pub async fn finish_login<'a>(gh_token: String, session: &Session<'a>) -> Result<(), Error> {
    let github = Client::new(
        "Lapce Extensions API v1",
        Credentials::Token(gh_token.clone().into()),
    );
    match github {
        Ok(github) => {
            let users = github.users();
            let user = users.get_authenticated().await.unwrap();
            let user = user.public_user().unwrap();
            if let Err(err) = session
                .set(SessionInfo {
                    gh_token,
                    id: user.id as u64,
                })
                .await
            {
                Err(Error {
                    kind: ErrorKind::DatabaseError(err.to_string()),
                    action: "Try again".into(),
                    message: "Can't set session on redis db".into(),
                })
            } else {
                let client = connect().await?;
                client
                    .user()
                    .upsert(
                        prisma::user::id::equals(user.id),
                        prisma::user::create(
                            user.id,
                            user.login.clone(),
                            user.name.clone(),
                            user.avatar_url.clone(),
                            vec![],
                        ),
                        vec![
                            prisma::user::name::set(user.login.clone()),
                            prisma::user::username::set(user.name.clone()),
                            prisma::user::avatar_url::set(user.avatar_url.clone()),
                        ],
                    )
                    .exec()
                    .await
                    .unwrap();
                Ok(())
            }
        }
        Err(err) => Err(Error {
            kind: ErrorKind::GithubApiError,
            action: "Try again later".into(),
            message: format!("Can't fetch from github api: {err}"),
        }),
    }
}
#[get("/auth/github")]
pub async fn callback(
    token: TokenResponse<GitHub>,
    session: Session<'_>,
) -> Result<Redirect, status::Custom<Json<Error>>> {
    let gh_token = token.access_token().to_string();
    finish_login(gh_token, &session)
        .await
        .map_err(|e| status::Custom(Status::InternalServerError, Json(e)))
        .map(|()| Redirect::to("/"))
}
#[post("/session/<token>")]
pub async fn login_with_token(
    token: String,
    session: Session<'_>,
) -> Result<(), status::Custom<Json<Error>>> {
    finish_login(token, &session)
        .await
        .map_err(|e| status::Custom(Status::InternalServerError, Json(e)))
}
