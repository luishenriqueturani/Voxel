CREATE TABLE compatibility_failures (
    model TEXT NOT NULL,
    backend TEXT NOT NULL,
    configuration TEXT NOT NULL,
    detail TEXT NOT NULL,
    recorded_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    PRIMARY KEY (model, backend, configuration)
);

CREATE TABLE setup_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
