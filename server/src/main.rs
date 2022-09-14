#[macro_use] extern crate rocket;
pub mod user;
mod github;
pub mod error;
use rocket::fs::FileServer;
use rocket_oauth2::OAuth2;
use crate::github::*;
use crate::user::get_user;
#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![github_callback, github_login])
        .mount("/api/", routes![get_user])
        .attach(OAuth2::<GitHub>::fairing("github"))
        .mount("/", FileServer::from("marketplace/dist"))
}
