CREATE TABLE generations (
    id INTEGER PRIMARY KEY,
    path TEXT NOT NULL,
    prompt TEXT NOT NULL,
    negative_prompt TEXT,
    model_id TEXT NOT NULL,
    backend TEXT NOT NULL,
    seed INTEGER NOT NULL,
    steps INTEGER NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    cfg REAL NOT NULL,
    elapsed_ms INTEGER NOT NULL,
    created_at TEXT NOT NULL
);
