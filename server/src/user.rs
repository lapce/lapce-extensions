use diesel::prelude::*;
use rocket::response::status::Unauthorized;
use rocket::serde::json::Json;
use rocket::serde::{Serialize, Deserialize};
use crate::Session;
use crate::db::establish_connection;
use crate::error::*;
#[derive(Deserialize, Serialize, Queryable, Insertable, AsChangeset, Clone)]
#[serde(crate = "rocket::serde")]
#[diesel(table_name = crate::schema::users)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub username: String,
    pub avatar_url: String
}

#[get("/user")]
pub async fn get_user(session: Session<'_>) -> Result<Json<User>, Unauthorized<Json<Error>>>{
    match session.get().await {
        Ok(Some(session)) => {

            use crate::schema::users::dsl::*;
            let connection = establish_connection();
            match connection {
                Ok(mut connection) => {
                    let user: User = users.find(session.id as i64).first(&mut connection).unwrap();
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
#[delete("/session")] 
pub async fn logout(session: Session<'_>) -> Result<(), Unauthorized<Json<Error>>> {
    if let None = session.get().await.unwrap(){
        Err(Unauthorized(Some(Json(Error {
            kind: ErrorKind::NotLoggedIn,
            action: "Send a `token` cookie.".into(),
            message: "You're already logged out".into()
        }))))
    } else {
        session.remove().await.unwrap();
        Ok(())
    }
}