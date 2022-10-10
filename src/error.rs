use rocket::serde::{Serialize, Deserialize};

#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub enum ErrorKind {
    NotLoggedIn,
    ValidationError,
    GithubApiError,
    DatabaseError(String)
}
#[derive(Deserialize, Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Error {
    pub(crate) kind: ErrorKind,
    pub(crate) action: String,
    pub(crate) message: String
}