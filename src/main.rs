#[macro_use]
extern crate rocket;
pub mod db;
pub mod error;
mod github;
pub mod repository;
pub mod user;
use crate::github::*;
use crate::user::get_user;
use dotenvy::dotenv;
use redis::Client;
use rocket::fairing::AdHoc;
use rocket::fs::FileServer;
use rocket::serde::{Deserialize, Serialize};
use rocket_oauth2::{HyperRustlsAdapter, OAuth2, OAuthConfig, StaticProvider};
pub use rocket_session_store::{redis::*, CookieConfig, SessionStore};
use std::time::Duration;
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(crate = "rocket::serde")]
pub struct SessionInfo {
    pub id: u64,
    pub gh_token: String,
}
pub type Session<'s> = rocket_session_store::Session<'s, SessionInfo>;
#[launch]
fn rocket() -> _ {
    dotenv().ok();
    let client: Client =
        Client::open(std::env::var("REDIS_URL").unwrap_or("redis://localhost".into()))
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
        .mount("/api/v1", routes![get_user, crate::user::logout])
        .attach(AdHoc::on_ignite("GitHub OAuth Config", |rocket| async {
            let config = OAuthConfig::new(
                StaticProvider::GitHub,
                std::env::var("GH_CLIENT_ID").unwrap(),
                std::env::var("GH_CLIENT_SECRET").unwrap(),
                Some(
                    std::env::var("GH_REDIRECT_URL")
                        .unwrap_or("https://localhost:8000/auth/github".into()),
                ),
            );
            rocket.attach(OAuth2::<GitHub>::custom(
                HyperRustlsAdapter::default(),
                config,
            ))
        }))
        .attach(store.fairing())
        .mount("/", FileServer::from("marketplace/dist"))
}
