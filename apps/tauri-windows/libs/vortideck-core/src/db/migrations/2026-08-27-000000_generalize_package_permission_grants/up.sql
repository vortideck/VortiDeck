-- Generalize the original widget-only consent ledger into the package-level
-- security boundary used by widgets, runtime plugins, and mixed bundles.
ALTER TABLE widget_packages RENAME TO package_permission_grants;
ALTER TABLE package_permission_grants RENAME COLUMN widget_id TO package_id;
