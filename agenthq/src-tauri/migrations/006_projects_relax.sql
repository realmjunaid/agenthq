CREATE TABLE projects_new (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    last_seen INTEGER
);
INSERT INTO projects_new SELECT id, name, path, last_seen FROM projects;
DROP TABLE projects;
ALTER TABLE projects_new RENAME TO projects;
