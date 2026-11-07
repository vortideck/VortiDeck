CREATE TABLE saved_actions_v2 (
    id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
    stable_id TEXT NOT NULL,
    integration_id TEXT NOT NULL,
    name TEXT NOT NULL,
    action TEXT NOT NULL,
    params TEXT NOT NULL
);

INSERT INTO saved_actions_v2 (id, stable_id, integration_id, name, action, params)
SELECT id, stable_id, integration_id, name, action, params FROM saved_actions;

DROP TABLE saved_actions;
ALTER TABLE saved_actions_v2 RENAME TO saved_actions;
CREATE UNIQUE INDEX saved_actions_stable_id_idx ON saved_actions(stable_id);
