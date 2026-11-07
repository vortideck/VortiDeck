-- Widget packages: user-approved widget grants (capability acceptance).
-- One row per installed widget package; widget_id is the PK.
--   manifest_json    — full manifest, for re-dialog diffs on update
--   permissions_json — the granted Permission set (compiled from capabilities)
--   content_hash     — sha256 of widget content, for verify-on-load (tamper detection)
--   trust            — "builtin" | "verified" | "community"
-- See docs/WIDGET_CAPABILITY_ARCHITECTURE.md §6.5 (threat model).
CREATE TABLE `widget_packages`(
    `widget_id` TEXT NOT NULL PRIMARY KEY,
    `manifest_json` TEXT NOT NULL,
    `permissions_json` TEXT NOT NULL,
    `content_hash` TEXT NOT NULL,
    `trust` TEXT NOT NULL,
    `granted_at` TEXT NOT NULL
);
