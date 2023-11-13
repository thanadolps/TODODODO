-- Add up migration script here
ALTER TABLE routine
ALTER COLUMN checktime
SET DEFAULT NOW();