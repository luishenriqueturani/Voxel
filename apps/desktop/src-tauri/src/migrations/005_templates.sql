CREATE TABLE templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    body TEXT NOT NULL,
    negative TEXT NOT NULL,
    model_2d TEXT NOT NULL,
    model_3d TEXT NOT NULL,
    resolution INTEGER NOT NULL,
    aspect TEXT NOT NULL,
    steps INTEGER NOT NULL,
    cfg REAL NOT NULL,
    seed TEXT NOT NULL,
    count INTEGER NOT NULL,
    postprocess INTEGER NOT NULL,
    created_at TEXT NOT NULL
);
