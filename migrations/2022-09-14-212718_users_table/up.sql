create table users (
    id bigint not null primary key,
    username varchar(39) not null,
    name varchar(255) not null,
    avatar_url varchar not null
);