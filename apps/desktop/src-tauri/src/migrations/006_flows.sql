CREATE TABLE flows (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    template_id TEXT,
    path TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE flow_items (
    id TEXT PRIMARY KEY,
    flow_id TEXT NOT NULL REFERENCES flows(id) ON DELETE CASCADE,
    position INTEGER NOT NULL,
    prompt TEXT NOT NULL,
    count INTEGER NOT NULL,
    mesh TEXT NOT NULL,
    overrides TEXT NOT NULL,
    UNIQUE (flow_id, position)
);
