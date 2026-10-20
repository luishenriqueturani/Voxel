CREATE TABLE favorites (
    id INTEGER PRIMARY KEY,
    prompt TEXT NOT NULL,
    negative_prompt TEXT NOT NULL DEFAULT '',
    model_id TEXT NOT NULL,
    steps INTEGER NOT NULL,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    cfg REAL NOT NULL,
    seed INTEGER NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (prompt, negative_prompt, model_id, steps, width, height, cfg)
);
