-- Add up migration script here
CREATE TABLE IF NOT EXISTS community_task (
    id UUID DEFAULT gen_random_uuid(),
    community_id UUID NOT NULL REFERENCES community(id) ON UPDATE CASCADE ON DELETE CASCADE,
    PRIMARY KEY (id, community_id),

    title TEXT NOT NULL,
    description TEXT NOT NULL,
    deadline TIMESTAMPTZ NULL,

    subtasks TEXT[] NOT NULL DEFAULT ARRAY[]::TEXT[]
);