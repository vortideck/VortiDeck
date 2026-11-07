CREATE TABLE saved_actions_v1 (
    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    integration_id TEXT NOT NULL,
    name TEXT NOT NULL,
    action TEXT NOT NULL,
    params TEXT NOT NULL
);

INSERT INTO saved_actions_v1 (id, integration_id, name, action, params)
SELECT id, integration_id, name, action, params FROM saved_actions;

DROP TABLE saved_actions;
ALTER TABLE saved_actions_v1 RENAME TO saved_actions;
