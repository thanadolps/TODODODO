-- Add up migration script here
CREATE TABLE IF NOT EXISTS invite_code (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    community_id UUID NOT NULL REFERENCES community(id),
    expired_at TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS invite_code_expired_at_idx ON invite_code(expired_at);