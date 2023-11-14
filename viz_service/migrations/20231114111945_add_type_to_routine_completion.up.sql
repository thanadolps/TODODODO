-- Add up migration script here
ALTER TABLE routine_completion
ADD COLUMN typena text NOT NULL;