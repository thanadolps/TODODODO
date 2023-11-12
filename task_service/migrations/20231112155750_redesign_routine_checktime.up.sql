-- Add up migration script here
ALTER TABLE habit
ALTER COLUMN score
SET DEFAULT 0;