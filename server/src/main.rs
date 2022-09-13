use rocket::fs::FileServer;

#[macro_use] extern crate rocket;

use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::serde::json::Json;
use rocket::serde::{Serialize, Deserialize};
use rocket::{Request, serde};
use rocket::response::Redirect;
use rocket_oauth2::{OAuth2, TokenResponse};

struct GitHub;

#[get("/login/github")]
fn github_login(oauth2: OAuth2<GitHub>, cookies: &CookieJar<'_>) -> Redirect {
    oauth2.get_redirect(&mut cookies, &["user:read"]).unwrap()
}
}
#[get("/auth/github")]
fn github_callback(token: TokenResponse<GitHub>, cookies: &CookieJar<'_>) -> Redirect
{
    cookies.add_private(
        Cookie::build("token", token.access_token().to_string())
            .same_site(SameSite::Lax)
            .finish()
    );
    Redirect::to("/")
}
#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![github_callback, github_login])
        .attach(OAuth2::<GitHub>::fairing("github"))
        .mount("/", FileServer::from("marketplace/dist"))
}
