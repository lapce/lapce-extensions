#[macro_use] extern crate rocket;
pub mod user;
mod github;
pub mod db;
pub mod error;
pub mod plugin;
pub mod schema;
use std::time::Duration;
use crate::plugin::*;
use dotenvy::dotenv;
use redis::Client;
use rocket::fairing::AdHoc;
use rocket::fs::FileServer;
use rocket::serde::{Deserialize, Serialize};
use rocket_oauth2::{OAuth2, HyperRustlsAdapter, StaticProvider, OAuthConfig};
use rocket_session_store::{redis::*, SessionStore, CookieConfig};
use crate::github::*;
use crate::user::get_user;
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "rocket::serde")]
pub struct SessionInfo {
    pub id: u64,
    pub gh_token: String
}
pub type Session<'s> = rocket_session_store::Session<'s, SessionInfo>;
#[launch]
fn rocket() -> _ {
    dotenv().ok();
    let client: Client = Client::open(std::env::var("REDIS_URL").unwrap_or("redis://localhost".into()))
	    .expect("Failed to connect to redis");
    let redis_store: RedisStore<SessionInfo> = RedisStore::new(client);
	let store: SessionStore<SessionInfo> = SessionStore {
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
        .mount("/api/", routes![get_user, crate::user::logout, publish_plugin, get_plugin_info, get_plugin_zip])
        .attach(AdHoc::on_ignite("GitHub OAuth Config", |rocket| async {
            let config = OAuthConfig::new(
                StaticProvider::GitHub,
                std::env::var("GH_CLIENT_ID").unwrap(),
                std::env::var("GH_CLIENT_SECRET").unwrap(),
                Some(std::env::var("GH_REDIRECT_URL").unwrap_or("https://localhost:8000/auth/github".into())),
            );
            rocket.attach(OAuth2::<GitHub>::custom(HyperRustlsAdapter::default(), config))
        }))
        .attach(store.fairing())
        .mount("/", FileServer::from("marketplace/dist"))
}
