-- A project's timeline holds meetings and emails; an email is stored as a meeting of kind
-- 'email' whose paragraphs are its transcript. stage groups the timeline ("Round 1", "Phase 2").
ALTER TABLE meetings ADD COLUMN kind TEXT NOT NULL DEFAULT 'meeting';

ALTER TABLE meetings ADD COLUMN stage TEXT;
