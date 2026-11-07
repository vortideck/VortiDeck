CREATE TABLE deck_item_persistent_state (
    item_id TEXT NOT NULL PRIMARY KEY,
    state_json TEXT NOT NULL DEFAULT '{}',
    updated_at TEXT NOT NULL,
    FOREIGN KEY(item_id) REFERENCES deck_items(id) ON DELETE CASCADE
);
