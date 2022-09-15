// @generated automatically by Diesel CLI.

diesel::table! {
    users (id) {
        id -> Int8,
        username -> Varchar,
        name -> Varchar,
        avatar_url -> Varchar,
    }
}
