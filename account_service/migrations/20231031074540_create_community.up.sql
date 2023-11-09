-- Add up migration script here
CREATE TABLE IF NOT EXISTS community (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    description TEXT NULL,
    is_private BOOLEAN NOT NULL DEFAULT FALSE,

    owner_id UUID NOT NULL REFERENCES account (id)
);

CREATE TABLE IF NOT EXISTS user_join_community (
    account_id UUID NOT NULL REFERENCES account (id),
    community_id UUID NOT NULL REFERENCES community (id),
    PRIMARY KEY (account_id, community_id)
); 