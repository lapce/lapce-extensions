create table plugins (
    id serial not null primary key,
    -- ID of the user that published this plugin
    user_id bigint not null,
    -- Name
    name varchar(255) not null,
    -- Plugin description
    description varchar(1024) not null,
    -- Extension version
    version varchar(50) not null,
    -- Display name of the plugin
    display_name varchar(80) not null,
    -- Repository URL
    repository varchar(600) not null,
    CONSTRAINT fk_user_id FOREIGN KEY (user_id) REFERENCES users (id)
)