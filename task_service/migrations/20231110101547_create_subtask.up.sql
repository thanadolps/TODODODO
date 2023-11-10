-- Add up migration script here
CREATE TABLE IF NOT EXISTS subtask (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    completed BOOL NOT NULL DEFAULT false,
    task_id UUID REFERENCES task(id) NOT NULL
);