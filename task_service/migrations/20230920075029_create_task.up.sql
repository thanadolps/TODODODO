BEGIN;
CREATE TABLE task (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    deadline TIMESTAMPTZ,
    completed BOOL NOT NULL DEFAULT false,
    user_id UUID NOT NULL,
    community_id UUID
);
CREATE INDEX IF NOT EXISTS task_deadline_idx ON task(deadline);
CREATE INDEX IF NOT EXISTS task_user_id_idx ON task(user_id);
END;