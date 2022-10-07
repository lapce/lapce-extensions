create table plugins (
    id serial not null primary key,
    -- ID of the user that published this plugin
    user_id bigint not null,
    CONSTRAINT fk_user_id FOREIGN KEY (user_id) REFERENCES users (id)
)