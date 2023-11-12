-- Add down migration script here
BEGIN;
-- Remove ON DELETE CASCADE from the foreign key reference for account_id
ALTER TABLE user_join_community
DROP CONSTRAINT IF EXISTS user_join_community_account_id_fkey,
ADD CONSTRAINT user_join_community_account_id_fkey
FOREIGN KEY (account_id) REFERENCES account (id);

-- Remove ON DELETE CASCADE from the foreign key reference for community_id
ALTER TABLE user_join_community
DROP CONSTRAINT IF EXISTS user_join_community_community_id_fkey,
ADD CONSTRAINT user_join_community_community_id_fkey
FOREIGN KEY (community_id) REFERENCES community (id);
END;