CREATE TABLE mcp_servers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    transport TEXT,
    command TEXT,
    url TEXT,
    source TEXT,
    status TEXT NOT NULL DEFAULT 'unknown',
    agent_id TEXT,
    project_id TEXT
);

CREATE TABLE skills (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    path TEXT,
    scope TEXT,
    source TEXT,
    agent_id TEXT
);

CREATE TABLE plugins (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT,
    path TEXT,
    source TEXT,
    enabled INTEGER NOT NULL DEFAULT 0,
    agent_id TEXT
);

CREATE TABLE models (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider TEXT,
    agent_id TEXT
);

CREATE TABLE connections (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider TEXT NOT NULL,
    configured INTEGER NOT NULL DEFAULT 0,
    agent_id TEXT
);

CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    path TEXT NOT NULL UNIQUE,
    last_seen INTEGER
);
