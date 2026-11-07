CREATE TABLE action_input_contracts (
    reference_key TEXT PRIMARY KEY NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision > 0),
    contract TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
