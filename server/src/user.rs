use diesel::prelude::*;
use rocket::http::CookieJar;
use rocket::response::status::Unauthorized;
use rocket::serde::json::Json;
use rocket::serde::{Serialize, Deserialize};
use rocket_session_store::Session;
use crate::db::establish_connection;
use crate::error::*;
#[derive(Deserialize, Serialize, Queryable, Insertable, AsChangeset)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub username: String,
    pub avatar_url: String
}
#[get("/user")]
pub async fn get_user(session: Session<'_, String>) -> Result<Json<User>, Unauthorized<Json<Error>>>{
    match session.get().await {
        Ok(Some(user_id)) => {

            use crate::schema::users::dsl::*;
            let connection = establish_connection();
            match connection {
                Ok(mut connection) => {
                    let user: User = users.find(user_id.parse::<i64>().unwrap()).first(&mut connection).unwrap();
                    Ok(Json(user))
                }
                Err(err) => {
                    Err(Unauthorized(Some(Json(err))))
                }
            }
        }
        _ => {
            Err(Unauthorized(Some(Json(Error {
                kind: ErrorKind::NotLoggedIn,
                action: "Send a `token` cookie.".into(),
                message: "Unauthorized".into()
            }))))
        }
    }
    
    
}