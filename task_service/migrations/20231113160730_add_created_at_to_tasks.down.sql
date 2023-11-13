-- Add down migration script here
ALTER TABLE task DROP COLUMN created_at;
ALTER TABLE routine DROP COLUMN created_at;
ALTER TABLE habit DROP COLUMN created_at;