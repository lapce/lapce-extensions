use crate::db::{connect, prisma::user};
use crate::error::*;
use crate::Session;
use rocket::response::status::Unauthorized;
use rocket::serde::json::Json;

#[get("/user")]
pub async fn get_user(session: Session<'_>) -> Result<Json<user::Data>, Unauthorized<Json<Error>>> {
    match session.get().await {
        Ok(Some(session)) => {
            let client = connect().await;
            match client {
                Ok(client) => {
                    let user = client
                        .user()
                        .find_unique(user::id::equals(session.id as i64))
                        .exec()
                        .await
                        .ok();
                    if let Some(Some(user)) = user {
                        Ok(Json(user))
                    } else {
                        Err(Unauthorized(Some(Json(Error {
                            kind: ErrorKind::NotLoggedIn,
                            action: "Send a `token` cookie.".into(),
                            message: "Unauthorized".into(),
                        }))))
                    }
                }
                Err(err) => Err(Unauthorized(Some(Json(err)))),
            }
        }
        _ => Err(Unauthorized(Some(Json(Error {
            kind: ErrorKind::NotLoggedIn,
            action: "Send a `token` cookie.".into(),
            message: "Unauthorized".into(),
        })))),
    }
}
#[delete("/session")]
pub async fn logout(session: Session<'_>) -> Result<(), Unauthorized<Json<Error>>> {
    if let None = session.get().await.unwrap() {
        Err(Unauthorized(Some(Json(Error {
            kind: ErrorKind::NotLoggedIn,
            action: "Send a `token` cookie.".into(),
            message: "You're already logged out".into(),
        }))))
    } else {
        session.remove().await.unwrap();
        Ok(())
    }
}
