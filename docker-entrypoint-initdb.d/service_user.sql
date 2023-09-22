DO $$ 
BEGIN 
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_user WHERE usename = 'task') THEN 
        CREATE USER task WITH PASSWORD 'password'; 
    END IF; 
END $$;
CREATE SCHEMA IF NOT EXISTS task;
GRANT ALL PRIVILEGES ON SCHEMA task TO task;


