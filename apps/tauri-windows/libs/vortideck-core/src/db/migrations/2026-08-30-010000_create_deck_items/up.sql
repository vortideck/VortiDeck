CREATE TABLE deck_widget_documents (
    deck_id INTEGER NOT NULL PRIMARY KEY,
    revision INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL,
    FOREIGN KEY(deck_id) REFERENCES decks(id) ON DELETE CASCADE
);

CREATE TABLE deck_items (
    id TEXT NOT NULL PRIMARY KEY,
    deck_id INTEGER NOT NULL,
    definition_id TEXT NOT NULL,
    package_id TEXT,
    contribution_id TEXT,
    config_json TEXT NOT NULL DEFAULT '{}',
    config_version INTEGER NOT NULL DEFAULT 1,
    appearance_json TEXT NOT NULL DEFAULT '{}',
    legacy_button_id INTEGER,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    FOREIGN KEY(deck_id) REFERENCES decks(id) ON DELETE CASCADE,
    FOREIGN KEY(legacy_button_id) REFERENCES buttons(id) ON DELETE SET NULL
);

CREATE TABLE deck_item_layouts (
    item_id TEXT NOT NULL,
    layout_key TEXT NOT NULL,
    x INTEGER NOT NULL,
    y INTEGER NOT NULL,
    col_span INTEGER NOT NULL,
    row_span INTEGER NOT NULL,
    PRIMARY KEY(item_id, layout_key),
    FOREIGN KEY(item_id) REFERENCES deck_items(id) ON DELETE CASCADE,
    CHECK(x >= 0 AND y >= 0 AND col_span > 0 AND row_span > 0)
);

CREATE TABLE deck_item_bindings (
    id TEXT NOT NULL PRIMARY KEY,
    item_id TEXT NOT NULL,
    port_id TEXT NOT NULL,
    action_json TEXT NOT NULL,
    policy_json TEXT NOT NULL,
    transform_json TEXT,
    FOREIGN KEY(item_id) REFERENCES deck_items(id) ON DELETE CASCADE
);

CREATE INDEX idx_deck_items_deck_id ON deck_items(deck_id);
CREATE INDEX idx_deck_items_legacy_button_id ON deck_items(legacy_button_id);
CREATE INDEX idx_deck_item_bindings_item_id ON deck_item_bindings(item_id);
