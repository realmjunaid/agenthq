CREATE TABLE agent_processes (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    pid INTEGER NOT NULL,
    parent_pid INTEGER,
    name TEXT NOT NULL,
    executable_path TEXT,
    cpu REAL NOT NULL DEFAULT 0.0,
    ram INTEGER NOT NULL DEFAULT 0,
    started_at INTEGER NOT NULL,
    UNIQUE(agent_id, pid)
);
CREATE INDEX idx_agent_processes_agent ON agent_processes(agent_id);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    project_id TEXT,
    status TEXT NOT NULL DEFAULT 'unknown',
    model TEXT,
    started_at INTEGER,
    last_activity INTEGER,
    confidence TEXT NOT NULL DEFAULT 'unknown'
);
CREATE INDEX idx_sessions_agent ON sessions(agent_id);

CREATE TABLE subagents (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    label TEXT,
    status TEXT NOT NULL DEFAULT 'unknown',
    started_at INTEGER
);

CREATE TABLE resource_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    ts INTEGER NOT NULL,
    cpu REAL NOT NULL DEFAULT 0.0,
    ram INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_resource_snapshots_agent_ts ON resource_snapshots(agent_id, ts);
