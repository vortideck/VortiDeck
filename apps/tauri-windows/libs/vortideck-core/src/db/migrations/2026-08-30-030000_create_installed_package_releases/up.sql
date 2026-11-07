CREATE TABLE installed_package_releases (
    package_id TEXT PRIMARY KEY NOT NULL,
    installed_version TEXT NOT NULL,
    source TEXT NOT NULL CHECK (source IN ('builtin', 'marketplace', 'local')),
    release_id TEXT,
    artifact_sha256 TEXT NOT NULL,
    marketplace_key_id TEXT,
    installed_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    auto_update BOOLEAN NOT NULL DEFAULT 1,
    lifecycle_state TEXT NOT NULL DEFAULT 'active',
    last_known_good_version TEXT
);

CREATE UNIQUE INDEX installed_package_release_id_unique
    ON installed_package_releases(release_id)
    WHERE release_id IS NOT NULL;

CREATE INDEX installed_package_source_state_idx
    ON installed_package_releases(source, lifecycle_state);
