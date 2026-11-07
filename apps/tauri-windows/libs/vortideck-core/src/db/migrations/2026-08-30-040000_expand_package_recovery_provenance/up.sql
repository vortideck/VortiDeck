ALTER TABLE installed_package_releases
    ADD COLUMN last_known_good_release_id TEXT;

ALTER TABLE installed_package_releases
    ADD COLUMN last_known_good_artifact_sha256 TEXT;

ALTER TABLE installed_package_releases
    ADD COLUMN last_known_good_marketplace_key_id TEXT;
