
use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::response::Redirect;
use rocket_oauth2::{OAuth2, TokenResponse};

pub struct GitHub;

#[get("/login/github")]
pub fn github_login(oauth2: OAuth2<GitHub>, cookies: &CookieJar<'_>) -> Redirect {
    oauth2.get_redirect(cookies, &["user:read"]).unwrap()
}

#[get("/auth/github")]
pub fn github_callback(token: TokenResponse<GitHub>, cookies: &CookieJar<'_>) -> Redirect
{
    cookies.add_private(
        Cookie::build("token", token.access_token().to_string())
            .same_site(SameSite::Lax)
            .finish()
    );
    Redirect::to("/")
}