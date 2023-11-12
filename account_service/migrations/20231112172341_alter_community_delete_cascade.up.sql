-- Add up migration script here
BEGIN;
-- Add ON DELETE CASCADE to the foreign key reference for account_id
ALTER TABLE user_join_community
DROP CONSTRAINT IF EXISTS user_join_community_account_id_fkey,
ADD CONSTRAINT user_join_community_account_id_fkey
FOREIGN KEY (account_id) REFERENCES account (id) ON DELETE CASCADE;

-- Add ON DELETE CASCADE to the foreign key reference for community_id
ALTER TABLE user_join_community
DROP CONSTRAINT IF EXISTS user_join_community_community_id_fkey,
ADD CONSTRAINT user_join_community_community_id_fkey
FOREIGN KEY (community_id) REFERENCES community (id) ON DELETE CASCADE;
END;