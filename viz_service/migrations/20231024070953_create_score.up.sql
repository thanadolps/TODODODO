-- Add up migration script here
CREATE TABLE IF NOT EXISTS performance (
    user_id UUID PRIMARY KEY,
    combo INT NOT NULL DEFAULT 0,
    best_record INT NOT NULL DEFAULT 0
)