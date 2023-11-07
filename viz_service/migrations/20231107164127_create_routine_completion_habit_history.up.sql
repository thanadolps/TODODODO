-- Add up migration script here
CREATE TABLE IF NOT EXISTS routine_completion (
    task_id UUID PRIMARY KEY,
    complete_date TIMESTAMPTZ NOT NULL
);
CREATE TABLE IF NOT EXISTS habit_history (
    task_id UUID PRIMARY KEY,
    positive BOOLEAN NOT NULL DEFAULT TRUE,
    triggered_at TIMESTAMPTZ NOT NULL
);