// @generated automatically by Diesel CLI.

diesel::table! {
    plugins (id) {
        id -> Int4,
        user_id -> Int8,
        name -> Varchar,
        description -> Varchar,
        version -> Varchar,
        display_name -> Varchar,
        repository -> Varchar,
    }
}

diesel::table! {
    users (id) {
        id -> Int8,
        username -> Varchar,
        name -> Varchar,
        avatar_url -> Varchar,
    }
}

diesel::joinable!(plugins -> users (user_id));

diesel::allow_tables_to_appear_in_same_query!(
    plugins,
    users,
);
