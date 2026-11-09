CREATE TABLE templates_prompt (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    body TEXT NOT NULL,
    negative TEXT NOT NULL,
    placement TEXT NOT NULL DEFAULT 'after',
    created_at TEXT NOT NULL
);
INSERT INTO templates_prompt (id, name, body, negative, placement, created_at)
SELECT id, name, body, negative, 'after', created_at FROM templates;
DROP TABLE templates;
ALTER TABLE templates_prompt RENAME TO templates;
