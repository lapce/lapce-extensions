use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use rocket::{response::status::Unauthorized, serde::json::Json};
use std::env;

use crate::error::{Error, ErrorKind};

pub fn establish_connection() -> Result<PgConnection, Error> {
    dotenv().ok();
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let res = PgConnection::establish(&database_url);
    match res {
        Ok(con) => {
            Ok(con)
        }
        Err(err) => {
            Err(Error{
                message: "Couldn't connect to the database".into(),
                action: "Try again later, or if you're the admin, check the database".into(),
                kind: ErrorKind::DatabaseError(err.to_string())
            })
        }
    }
}