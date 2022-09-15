#[macro_use] extern crate rocket;
pub mod user;
mod github;
pub mod db;
pub mod error;
pub mod schema;
use std::time::Duration;

use dotenvy::dotenv;
use redis::Client;
use rocket::fs::FileServer;
use rocket_oauth2::OAuth2;
use rocket_session_store::memory::MemoryStore;
use rocket_session_store::{redis::*, SessionStore, CookieConfig};
use crate::github::*;
use crate::user::get_user;
#[launch]
fn rocket() -> _ {
    dotenv().ok();
    let client: Client = Client::open(std::env::var("REDIS_URL").unwrap_or("redis://localhost".into()))
	    .expect("Failed to connect to redis");
    let redis_store: RedisStore<String> = RedisStore::new(client);
	let store: SessionStore<String> = SessionStore {
		store: Box::new(redis_store),
		name: "token".into(), 

		duration: Duration::from_secs(3600 * 24 * 3),
		cookie: CookieConfig {
            http_only: false,
            secure: true,
            path: Some("/".into()),
            ..Default::default()
        },
	};
    rocket::build()
        .mount("/", routes![github_callback, github_login])
        .mount("/api/", routes![get_user, crate::user::logout])
        .attach(OAuth2::<GitHub>::fairing("github"))
        .attach(store.fairing())
        .mount("/", FileServer::from("marketplace/dist"))
}
