-- Projects group meetings by client/project. A meeting has at most one project;
-- deleting a project leaves its meetings in place, unassigned.
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TEXT NOT NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_projects_name ON projects (name COLLATE NOCASE);

ALTER TABLE meetings ADD COLUMN project_id TEXT REFERENCES projects (id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_meetings_project_id ON meetings (project_id);
