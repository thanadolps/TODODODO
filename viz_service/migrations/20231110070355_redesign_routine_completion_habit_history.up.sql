-- Add up migration script here
-- Remove the previous version
DROP TABLE IF EXISTS routine_completion;
DROP TABLE IF EXISTS habit_history;
-- Declare the new design
CREATE TABLE IF NOT EXISTS routine_completion (
    task_id UUID NOT NULL,
    completed_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (task_id, completed_at)
);
CREATE TABLE IF NOT EXISTS habit_history (
    task_id UUID NOT NULL,
    positive BOOLEAN NOT NULL DEFAULT TRUE,
    triggered_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (task_id, triggered_at)
);