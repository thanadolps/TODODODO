-- Add up migration script here
CREATE TABLE IF NOT EXISTS webhook (
    user_id UUID PRIMARY KEY NOT NULL,
    url TEXT NULL
);