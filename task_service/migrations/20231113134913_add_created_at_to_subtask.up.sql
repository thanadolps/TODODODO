-- Add up migration script here
ALTER TABLE subtask
ADD COLUMN created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP;