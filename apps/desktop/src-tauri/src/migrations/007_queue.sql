CREATE TABLE queue_jobs (
    id TEXT PRIMARY KEY,
    flow_id TEXT,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE queue_items (
    id TEXT PRIMARY KEY,
    job_id TEXT NOT NULL REFERENCES queue_jobs(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    flow_item_id TEXT NOT NULL,
    status TEXT NOT NULL,
    mesh TEXT NOT NULL,
    count INTEGER NOT NULL,
    images INTEGER NOT NULL,
    meshes INTEGER NOT NULL,
    resolved TEXT NOT NULL,
    error_log TEXT,
    error_summary TEXT
);

CREATE TABLE queue_outputs (
    id TEXT PRIMARY KEY,
    item_id TEXT NOT NULL REFERENCES queue_items(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    path TEXT NOT NULL,
    seed INTEGER,
    position INTEGER NOT NULL
);
