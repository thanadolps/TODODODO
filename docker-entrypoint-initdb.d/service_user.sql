DO $$ 
BEGIN 
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_user WHERE usename = 'task') THEN 
        CREATE USER task WITH PASSWORD 'password'; 
    END IF; 
END $$;
CREATE SCHEMA IF NOT EXISTS task;
GRANT ALL PRIVILEGES ON SCHEMA task TO task;

DO $$ 
BEGIN 
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_user WHERE usename = 'score') THEN 
        CREATE USER score WITH PASSWORD 'password'; 
    END IF; 
END $$;
CREATE SCHEMA IF NOT EXISTS score;
GRANT ALL PRIVILEGES ON SCHEMA score TO score;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_user WHERE usename = 'account') THEN 
        CREATE USER account WITH PASSWORD 'password'; 
    END IF; 
END $$;
CREATE SCHEMA IF NOT EXISTS "account";
GRANT ALL PRIVILEGES ON SCHEMA "account" TO account;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT FROM pg_catalog.pg_user WHERE usename = 'notification') THEN 
        CREATE USER notification WITH PASSWORD 'password'; 
    END IF; 
END $$;
CREATE SCHEMA IF NOT EXISTS "notification";
GRANT ALL PRIVILEGES ON SCHEMA "notification" TO notification;
