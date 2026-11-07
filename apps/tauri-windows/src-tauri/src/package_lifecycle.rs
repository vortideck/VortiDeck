use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use tauri::{AppHandle, Emitter, Manager};
use vortideck_core::db::database::Database;
use vortideck_core::db::db::DbPool;
use vortideck_core::db::models::{
    InstalledPackageRelease, NewInstalledPackageRelease, NewPackagePermissionGrant,
    PackagePermissionGrant,
};
use vortideck_core::plugins::{
    canonical_package_id, compile_manifest_permissions, manifest_consent_hash, PluginChangeEvent,
};
use vortideck_core::VortiDeckCore;

static LIFECYCLE_LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();

fn lifecycle_lock() -> &'static tokio::sync::Mutex<()> {
    LIFECYCLE_LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PackageLifecyclePhase {
    Downloading,
    Verifying,
    Staging,
    Inspecting,
    AwaitingReview,
    Updating,
    Compiling,
    Active,
    GrantedInactive,
    RollingBack,
    RolledBack,
    Unavailable,
    Uninstalling,
    Uninstalled,
    Failed,
}

#[derive(Clone, Debug, Serialize)]
pub struct PackageLifecycleEvent {
    pub package_id: String,
    pub phase: PackageLifecyclePhase,
    pub progress: u8,
    pub message: String,
    pub error: Option<String>,
    pub needs_review: bool,
    pub runtime_active: bool,
}

fn emit(
    app: &AppHandle,
    package_id: &str,
    phase: PackageLifecyclePhase,
    progress: u8,
    message: impl Into<String>,
    error: Option<String>,
    needs_review: bool,
    runtime_active: bool,
) {
    let _ = app.emit(
        "package-lifecycle",
        PackageLifecycleEvent {
            package_id: canonical_package_id(package_id).to_string(),
            phase,
            progress,
            message: message.into(),
            error,
            needs_review,
            runtime_active,
        },
    );
}

/// End all cached transport authority for a package. Native iframe principals
/// are removed synchronously; overlay state/client cleanup runs on Tauri's
/// runtime and updates authoritative HTML before notifying connected sources.
pub async fn invalidate_package_authority(app: &AppHandle, package_id: &str) {
    let canonical = canonical_package_id(package_id).to_string();
    if let Some(registry) =
        app.try_state::<crate::commands::plugin_commands::PluginFrameSessionRegistry>()
    {
        if let Err(error) = registry.revoke_package(&canonical) {
            log::warn!(
                "Failed to revoke live frame sessions for package '{}': {}",
                canonical,
                error
            );
        }
    }
    let _ = app.emit(
        "package-authority-changed",
        serde_json::json!({ "packageId": canonical.clone() }),
    );
    if let Err(error) =
        crate::commands::overlay_commands::invalidate_overlay_package(app, &canonical).await
    {
        log::warn!(
            "Failed to invalidate overlay frames for package '{}': {}",
            canonical,
            error
        );
    }
}

/// Re-enable only the overlay projection after lifecycle code has established
/// current authority. Native iframe sessions are intentionally not revived;
/// untrusted plugin windows must register a fresh source-bound session.
async fn restore_package_authority(app: &AppHandle, package_id: &str) {
    if let Err(error) =
        crate::commands::overlay_commands::restore_overlay_package(app, package_id).await
    {
        log::warn!(
            "Failed to restore overlay frames for package '{}': {}",
            canonical_package_id(package_id),
            error
        );
    }
}

pub fn report(
    app: &AppHandle,
    package_id: &str,
    phase: PackageLifecyclePhase,
    progress: u8,
    message: impl Into<String>,
    error: Option<String>,
    needs_review: bool,
    runtime_active: bool,
) {
    emit(
        app,
        package_id,
        phase,
        progress,
        message,
        error,
        needs_review,
        runtime_active,
    );
}

fn matching_grant<'a>(
    grants: &'a [PackagePermissionGrant],
    package_id: &str,
) -> Option<&'a PackagePermissionGrant> {
    let canonical = canonical_package_id(package_id);
    grants
        .iter()
        .find(|grant| canonical_package_id(&grant.package_id) == canonical)
}

fn grant_matches(
    grant: Option<&PackagePermissionGrant>,
    manifest: &vortideck_core::plugins::BundleManifest,
) -> Result<bool, String> {
    let Some(grant) = grant else {
        return Ok(false);
    };
    let permissions = compile_manifest_permissions(manifest);
    let permissions_json = serde_json::to_string(&permissions)
        .map_err(|error| format!("Failed to serialize package permissions: {error}"))?;
    Ok(grant.permissions_json == permissions_json
        && grant.content_hash == manifest_consent_hash(manifest)?)
}

fn backup_root() -> Result<PathBuf, String> {
    VortiDeckCore::get_default_plugins_directory()
        .parent()
        .map(|parent| parent.join("plugin-runtime-backups"))
        .ok_or_else(|| "Plugin directory has no parent for runtime backups".to_string())
}

fn safe_id(id: &str) -> String {
    canonical_package_id(id)
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn backup_path(package_id: &str) -> Result<PathBuf, String> {
    Ok(backup_root()?.join(safe_id(package_id)))
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir_all(destination)
        .map_err(|error| format!("Failed to create {}: {error}", destination.display()))?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("Failed to read {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("Failed to read package entry: {error}"))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Failed to inspect {}: {error}", source_path.display()))?;
        if file_type.is_symlink() {
            return Err(format!(
                "Package backups cannot follow symbolic links: {}",
                source_path.display()
            ));
        }
        if file_type.is_dir() {
            copy_directory(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path).map_err(|error| {
                format!(
                    "Failed to copy {} to {}: {error}",
                    source_path.display(),
                    destination_path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn hash_directory(root: &Path) -> Result<Vec<u8>, String> {
    fn collect(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in fs::read_dir(current)
            .map_err(|error| format!("Failed to read {}: {error}", current.display()))?
        {
            let entry = entry.map_err(|error| format!("Failed to read package entry: {error}"))?;
            let file_type = entry
                .file_type()
                .map_err(|error| format!("Failed to inspect package artifact: {error}"))?;
            if file_type.is_symlink() {
                return Err(format!(
                    "Package artifacts cannot contain symbolic links: {}",
                    entry.path().display()
                ));
            }
            if file_type.is_dir() {
                collect(root, &entry.path(), files)?;
            } else if file_type.is_file() {
                files.push(entry.path());
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    collect(root, root, &mut files)?;
    files.sort_by(|left, right| {
        left.strip_prefix(root)
            .unwrap_or(left)
            .cmp(right.strip_prefix(root).unwrap_or(right))
    });
    let mut hasher = Sha256::new();
    for file in files {
        let relative = file.strip_prefix(root).unwrap_or(&file);
        hasher.update(relative.to_string_lossy().as_bytes());
        hasher.update([0]);
        let bytes = fs::read(&file)
            .map_err(|error| format!("Failed to hash {}: {error}", file.display()))?;
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    Ok(hasher.finalize().to_vec())
}

fn matches_last_known_good_sync(package_id: &str, source: &Path) -> Result<bool, String> {
    let backup = backup_path(package_id)?;
    if !backup.is_dir() {
        return Ok(false);
    }
    Ok(hash_directory(source)? == hash_directory(&backup)?)
}

async fn matches_last_known_good(package_id: &str, source: &Path) -> Result<bool, String> {
    let package_id = package_id.to_string();
    let source = source.to_path_buf();
    tokio::task::spawn_blocking(move || matches_last_known_good_sync(&package_id, &source))
        .await
        .map_err(|error| format!("Package hash task failed: {error}"))?
}

async fn load_runtime_with_timeout(core: &VortiDeckCore, folder: &Path) -> Result<String, String> {
    tokio::time::timeout(
        std::time::Duration::from_secs(30),
        core.plugin_manager().load_plugin_from_folder(folder),
    )
    .await
    .map_err(|_| {
        format!(
            "Package runtime activation timed out after 30s: {}",
            folder.display()
        )
    })?
}

/// Replace the recoverable last-known-good snapshot only after activation has
/// succeeded. The backup lives outside the watched plugins directory.
fn record_last_known_good_sync(package_id: &str, source: &Path) -> Result<(), String> {
    if !source.is_dir() {
        return Err(format!(
            "Package folder does not exist: {}",
            source.display()
        ));
    }
    let destination = backup_path(package_id)?;
    let staging = destination.with_extension("staging");
    let previous = destination.with_extension("previous");
    if staging.exists() {
        fs::remove_dir_all(&staging)
            .map_err(|error| format!("Failed to clean stale backup staging: {error}"))?;
    }
    if previous.exists() {
        fs::remove_dir_all(&previous)
            .map_err(|error| format!("Failed to clean stale previous backup: {error}"))?;
    }
    copy_directory(source, &staging)?;
    if destination.exists() {
        fs::rename(&destination, &previous)
            .map_err(|error| format!("Failed to stage previous package backup: {error}"))?;
    }
    if let Err(error) = fs::rename(&staging, &destination) {
        if previous.exists() {
            let _ = fs::rename(&previous, &destination);
        }
        return Err(format!("Failed to commit package backup: {error}"));
    }
    if previous.exists() {
        fs::remove_dir_all(&previous)
            .map_err(|error| format!("Failed to clean replaced package backup: {error}"))?;
    }
    Ok(())
}

/// Copying and hashing a package can be substantial work. Keep it off Tokio's
/// async worker threads while retaining synchronous, atomic filesystem steps.
pub async fn record_last_known_good(package_id: &str, source: &Path) -> Result<(), String> {
    let package_id = package_id.to_string();
    let source = source.to_path_buf();
    tokio::task::spawn_blocking(move || record_last_known_good_sync(&package_id, &source))
        .await
        .map_err(|error| format!("Package backup task failed: {error}"))?
}

fn find_package_folder_sync(package_id: &str) -> Result<Option<PathBuf>, String> {
    let canonical = canonical_package_id(package_id);
    let plugins_dir = VortiDeckCore::get_default_plugins_directory();
    let entries = match fs::read_dir(&plugins_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Failed to scan package directory {}: {error}",
                plugins_dir.display()
            ));
        }
    };
    for entry in entries {
        let path = entry
            .map_err(|error| format!("Failed to inspect package directory: {error}"))?
            .path();
        if !path.is_dir() {
            continue;
        }
        let manifest_path = path.join("manifest.json");
        let Ok(json) = fs::read_to_string(&manifest_path) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_str::<vortideck_core::plugins::BundleManifest>(&json)
        else {
            continue;
        };
        if canonical_package_id(&manifest.id) == canonical {
            return Ok(Some(path));
        }
    }
    Ok(None)
}

async fn find_package_folder(package_id: &str) -> Result<Option<PathBuf>, String> {
    let package_id = package_id.to_string();
    tokio::task::spawn_blocking(move || find_package_folder_sync(&package_id))
        .await
        .map_err(|error| format!("Package discovery task failed: {error}"))?
}

async fn mark_installed_receipt_active(
    database: &Database,
    package_id: &str,
) -> Result<(), String> {
    let Some(receipt) = database.get_installed_package_release(package_id).await? else {
        return Ok(());
    };
    let now = chrono::Utc::now().to_rfc3339();
    database
        .save_installed_package_release(NewInstalledPackageRelease {
            package_id: &receipt.package_id,
            installed_version: &receipt.installed_version,
            source: &receipt.source,
            release_id: receipt.release_id.as_deref(),
            artifact_sha256: &receipt.artifact_sha256,
            marketplace_key_id: receipt.marketplace_key_id.as_deref(),
            installed_at: &receipt.installed_at,
            updated_at: &now,
            auto_update: receipt.auto_update,
            lifecycle_state: "active",
            last_known_good_version: Some(&receipt.installed_version),
            last_known_good_release_id: receipt.release_id.as_deref(),
            last_known_good_artifact_sha256: Some(&receipt.artifact_sha256),
            last_known_good_marketplace_key_id: receipt.marketplace_key_id.as_deref(),
            installed_content_sha256: receipt.installed_content_sha256.as_deref(),
        })
        .await
}

async fn preserve_installed_receipt_with_state(
    database: &Database,
    receipt: &InstalledPackageRelease,
    lifecycle_state: &str,
) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    database
        .save_installed_package_release(NewInstalledPackageRelease {
            package_id: &receipt.package_id,
            installed_version: &receipt.installed_version,
            source: &receipt.source,
            release_id: receipt.release_id.as_deref(),
            artifact_sha256: &receipt.artifact_sha256,
            marketplace_key_id: receipt.marketplace_key_id.as_deref(),
            installed_at: &receipt.installed_at,
            updated_at: &now,
            auto_update: receipt.auto_update,
            lifecycle_state,
            last_known_good_version: receipt.last_known_good_version.as_deref(),
            last_known_good_release_id: receipt.last_known_good_release_id.as_deref(),
            last_known_good_artifact_sha256: receipt.last_known_good_artifact_sha256.as_deref(),
            last_known_good_marketplace_key_id: receipt
                .last_known_good_marketplace_key_id
                .as_deref(),
            installed_content_sha256: receipt.installed_content_sha256.as_deref(),
        })
        .await
}

/// Repair durable receipts left between filesystem publication and lifecycle
/// completion. This runs before startup discovery so a process kill cannot
/// leave an install permanently labelled as in progress.
pub async fn recover_interrupted_operations(db_pool: &DbPool) -> Result<(), String> {
    let _guard = lifecycle_lock().lock().await;
    let database = Database::new(db_pool);
    for receipt in database.get_all_installed_package_releases().await? {
        if !matches!(receipt.lifecycle_state.as_str(), "installing" | "updating") {
            continue;
        }

        let folder = find_package_folder(&receipt.package_id).await?;
        let Some(folder) = folder else {
            if receipt.lifecycle_state == "installing" {
                database
                    .remove_installed_package_release(&receipt.package_id)
                    .await?;
            } else {
                preserve_installed_receipt_with_state(&database, &receipt, "failed").await?;
            }
            continue;
        };

        let content_matches = match receipt.installed_content_sha256.as_deref() {
            Some(expected) => {
                let actual =
                    tokio::task::spawn_blocking(move || hash_directory(&folder).map(hex::encode))
                        .await
                        .map_err(|error| format!("Package recovery hash task failed: {error}"))??;
                actual.eq_ignore_ascii_case(expected)
            }
            None => false,
        };
        preserve_installed_receipt_with_state(
            &database,
            &receipt,
            if content_matches {
                "awaiting_review"
            } else {
                "failed"
            },
        )
        .await?;
    }
    Ok(())
}

async fn restore_receipt_to_last_known_good(
    database: &Database,
    package_id: &str,
) -> Result<bool, String> {
    let Some(receipt) = database.get_installed_package_release(package_id).await? else {
        return Ok(false);
    };
    let Some(version) = receipt.last_known_good_version.as_deref() else {
        return Ok(false);
    };
    let Some(artifact_sha256) = receipt.last_known_good_artifact_sha256.as_deref() else {
        return Ok(false);
    };
    let now = chrono::Utc::now().to_rfc3339();
    database
        .save_installed_package_release(NewInstalledPackageRelease {
            package_id: &receipt.package_id,
            installed_version: version,
            source: &receipt.source,
            release_id: receipt.last_known_good_release_id.as_deref(),
            artifact_sha256,
            marketplace_key_id: receipt.last_known_good_marketplace_key_id.as_deref(),
            installed_at: &receipt.installed_at,
            updated_at: &now,
            auto_update: receipt.auto_update,
            lifecycle_state: "rolled_back",
            last_known_good_version: Some(version),
            last_known_good_release_id: receipt.last_known_good_release_id.as_deref(),
            last_known_good_artifact_sha256: Some(artifact_sha256),
            last_known_good_marketplace_key_id: receipt
                .last_known_good_marketplace_key_id
                .as_deref(),
            installed_content_sha256: receipt.installed_content_sha256.as_deref(),
        })
        .await?;
    Ok(true)
}

/// The sole desktop authority for granting consent and activating a package.
/// The Tauri command is intentionally a thin adapter so startup, watcher, UI,
/// and retry paths cannot grow separate activation state machines.
pub async fn grant_package(
    app: &AppHandle,
    core: &VortiDeckCore,
    db_pool: &DbPool,
    package_id: &str,
) -> Result<usize, String> {
    let _guard = lifecycle_lock().lock().await;
    invalidate_package_authority(app, package_id).await;
    emit(
        app,
        package_id,
        PackageLifecyclePhase::Inspecting,
        10,
        "Validating package access…",
        None,
        false,
        core.plugin_manager()
            .is_package_runtime_loaded(package_id)
            .await,
    );

    let plugin_manager = core.plugin_manager();
    let manifest = plugin_manager
        .package_manifest(package_id)
        .await
        .ok_or_else(|| format!("No discovered manifest for package '{package_id}'"))?;
    let permissions = compile_manifest_permissions(&manifest);
    let permissions_json = serde_json::to_string(&permissions)
        .map_err(|error| format!("Failed to serialize package permissions: {error}"))?;
    let manifest_json = serde_json::to_string(&manifest)
        .map_err(|error| format!("Failed to serialize package manifest: {error}"))?;
    let content_hash = manifest_consent_hash(&manifest)?;
    let trust = if vortideck_core::plugins::integrity::is_known_builtin(&manifest.id) {
        "builtin"
    } else {
        "community"
    };
    let granted_at = chrono::Utc::now().to_rfc3339();
    let database = Database::new(db_pool);
    let previous_grants: Vec<_> = database
        .get_all_package_permission_grants()
        .await?
        .into_iter()
        .filter(|grant| {
            canonical_package_id(&grant.package_id) == canonical_package_id(&manifest.id)
        })
        .collect();
    let previous_permissions = previous_grants.first().and_then(|grant| {
        serde_json::from_str::<Vec<vortideck_core::plugins::PluginPermission>>(
            &grant.permissions_json,
        )
        .ok()
    });
    let previous_grant_ids = previous_grants
        .iter()
        .map(|grant| grant.package_id.clone())
        .collect::<Vec<_>>();
    database
        .remove_package_permission_grants(&previous_grant_ids)
        .await?;
    database
        .save_package_permission_grant(NewPackagePermissionGrant {
            package_id: &manifest.id,
            manifest_json: &manifest_json,
            permissions_json: &permissions_json,
            content_hash: &content_hash,
            trust,
            granted_at: &granted_at,
        })
        .await?;
    plugin_manager
        .get_permission_checker()
        .grant_permissions(&manifest.id, permissions.clone());

    let Some(folder) = find_package_folder(&manifest.id).await? else {
        mark_installed_receipt_active(&database, &manifest.id).await?;
        restore_package_authority(app, &manifest.id).await;
        emit(
            app,
            &manifest.id,
            PackageLifecyclePhase::Active,
            100,
            "Package access saved",
            None,
            false,
            false,
        );
        return Ok(permissions.len());
    };
    if database
        .get_installed_package_release(&manifest.id)
        .await?
        .is_none()
    {
        let folder_for_hash = folder.clone();
        let artifact_sha256 =
            tokio::task::spawn_blocking(move || hash_directory(&folder_for_hash).map(hex::encode))
                .await
                .map_err(|error| format!("Package provenance hash task failed: {error}"))??;
        let now = chrono::Utc::now().to_rfc3339();
        database
            .save_installed_package_release(NewInstalledPackageRelease {
                package_id: &manifest.id,
                installed_version: &manifest.version,
                source: "local",
                release_id: None,
                artifact_sha256: &artifact_sha256,
                marketplace_key_id: None,
                installed_at: &now,
                updated_at: &now,
                auto_update: false,
                lifecycle_state: "awaiting_review",
                last_known_good_version: None,
                last_known_good_release_id: None,
                last_known_good_artifact_sha256: None,
                last_known_good_marketplace_key_id: None,
                installed_content_sha256: Some(&artifact_sha256),
            })
            .await?;
    }
    if !VortiDeckCore::plugin_folder_has_runtime(&folder)? {
        mark_installed_receipt_active(&database, &manifest.id).await?;
        restore_package_authority(app, &manifest.id).await;
        emit(
            app,
            &manifest.id,
            PackageLifecyclePhase::Active,
            100,
            "Package contributions installed",
            None,
            false,
            false,
        );
        return Ok(permissions.len());
    }

    if plugin_manager.is_package_runtime_loaded(&manifest.id).await {
        if let Err(error) = record_last_known_good(&manifest.id, &folder).await {
            log::warn!(
                "Package '{}' is active, but its recovery snapshot failed: {}",
                manifest.id,
                error
            );
        }
    } else {
        let allowed = plugin_manager.get_allowed_plugin_count().await as usize;
        if plugin_manager.get_loaded_package_count().await >= allowed {
            let error = format!(
                "Access was granted, but runtime activation is deferred because the plugin limit ({allowed}) is reached"
            );
            emit(
                app,
                &manifest.id,
                PackageLifecyclePhase::GrantedInactive,
                100,
                "Package granted but runtime is inactive",
                Some(error.clone()),
                false,
                false,
            );
            restore_package_authority(app, &manifest.id).await;
            return Err(error);
        }
        emit(
            app,
            &manifest.id,
            PackageLifecyclePhase::Compiling,
            55,
            "Access saved; compiling and activating runtime…",
            None,
            false,
            false,
        );
        if let Err(error) = load_runtime_with_timeout(core, &folder).await {
            let activation_error = format!(
                "Access was granted, but package '{}' could not be activated: {error}",
                manifest.id
            );
            database
                .remove_package_permission_grant(&manifest.id)
                .await?;
            database
                .restore_package_permission_grants(&previous_grants)
                .await?;
            let error =
                match rollback_runtime(app, core, &manifest.id, previous_permissions.as_deref())
                    .await
                {
                    Ok(true) => {
                        restore_receipt_to_last_known_good(&database, &manifest.id).await?;
                        restore_package_authority(app, &manifest.id).await;
                        format!("{activation_error}; the previous verified version was restored")
                    }
                    Ok(false) => activation_error,
                    Err(rollback_error) => {
                        format!("{activation_error}; rollback failed: {rollback_error}")
                    }
                };
            emit(
                app,
                &manifest.id,
                PackageLifecyclePhase::GrantedInactive,
                100,
                "Package granted but activation failed",
                Some(error.clone()),
                false,
                false,
            );
            return Err(error);
        }
        if let Err(error) = record_last_known_good(&manifest.id, &folder).await {
            log::warn!(
                "Package '{}' activated, but its recovery snapshot failed: {}",
                manifest.id,
                error
            );
        }
    }

    emit(
        app,
        &manifest.id,
        PackageLifecyclePhase::Active,
        100,
        "Package installed and active",
        None,
        false,
        plugin_manager.is_package_runtime_loaded(&manifest.id).await,
    );
    mark_installed_receipt_active(&database, &manifest.id).await?;
    restore_package_authority(app, &manifest.id).await;
    Ok(permissions.len())
}

async fn rollback_runtime(
    app: &AppHandle,
    core: &VortiDeckCore,
    package_id: &str,
    restored_permissions: Option<&[vortideck_core::plugins::PluginPermission]>,
) -> Result<bool, String> {
    let backup = backup_path(package_id)?;
    if !backup.join("manifest.json").exists() {
        return Ok(false);
    }
    emit(
        app,
        package_id,
        PackageLifecyclePhase::RollingBack,
        82,
        "Restoring the last working runtime…",
        None,
        false,
        false,
    );
    core.plugin_manager()
        .unload_package_runtime(package_id)
        .await?;
    let current = find_package_folder(package_id).await?;
    let destination = current.unwrap_or_else(|| {
        VortiDeckCore::get_default_plugins_directory().join(safe_id(package_id))
    });
    let backup_for_task = backup.clone();
    let destination_for_task = destination.clone();
    tokio::task::spawn_blocking(move || {
        let parent = destination_for_task
            .parent()
            .ok_or_else(|| "Package folder has no parent".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create plugins directory: {error}"))?;
        let nonce = uuid::Uuid::new_v4();
        let staged = parent.join(format!(".rollback-staging-{nonce}"));
        let rejected = parent.join(format!(".rollback-rejected-{nonce}"));
        copy_directory(&backup_for_task, &staged)?;
        if destination_for_task.exists() {
            fs::rename(&destination_for_task, &rejected)
                .map_err(|error| format!("Failed to stage rejected package: {error}"))?;
        }
        if let Err(error) = fs::rename(&staged, &destination_for_task) {
            if rejected.exists() {
                let _ = fs::rename(&rejected, &destination_for_task);
            }
            let _ = fs::remove_dir_all(&staged);
            return Err(format!("Failed to publish rollback package: {error}"));
        }
        if rejected.exists() {
            fs::remove_dir_all(&rejected)
                .map_err(|error| format!("Failed to clean rejected update: {error}"))?;
        }
        Ok::<(), String>(())
    })
    .await
    .map_err(|error| format!("Package rollback task failed: {error}"))??;
    core.plugin_manager()
        .discover_plugin_from_folder(&destination)
        .await
        .map_err(|error| format!("Restored package discovery failed: {error}"))?;
    if let Some(permissions) = restored_permissions {
        core.plugin_manager()
            .get_permission_checker()
            .grant_permissions(package_id, permissions.to_vec());
    }
    load_runtime_with_timeout(core, &destination)
        .await
        .map_err(|error| format!("Last-known-good rollback failed: {error}"))?;
    emit(
        app,
        package_id,
        PackageLifecyclePhase::RolledBack,
        100,
        "Update failed; the previous runtime was restored",
        None,
        false,
        true,
    );
    Ok(true)
}

/// Reconcile one watcher candidate. Returns true when flow/UI providers must be
/// rebuilt. The watcher itself is intentionally unable to mutate runtimes.
pub async fn reconcile_file_change(
    app: &AppHandle,
    core: &VortiDeckCore,
    db_pool: &DbPool,
    change: &PluginChangeEvent,
) -> Result<bool, String> {
    let _guard = lifecycle_lock().lock().await;
    let folder = PathBuf::from(&change.plugin_path);
    let package_id = change.plugin_id.clone().unwrap_or_else(|| {
        folder
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    });
    invalidate_package_authority(app, &package_id).await;
    emit(
        app,
        &package_id,
        PackageLifecyclePhase::Inspecting,
        10,
        "Inspecting package change…",
        None,
        false,
        core.plugin_manager()
            .is_package_runtime_loaded(&package_id)
            .await,
    );

    if change.event_type == "removed" && !folder.join("manifest.json").exists() {
        core.plugin_manager()
            .unload_package_runtime(&package_id)
            .await?;
        core.plugin_manager()
            .remove_discovered_package(&package_id)
            .await;
        emit(
            app,
            &package_id,
            PackageLifecyclePhase::Unavailable,
            100,
            "Package files were removed; saved references remain unavailable",
            None,
            false,
            false,
        );
        return Ok(true);
    }

    let manifest = match core
        .plugin_manager()
        .discover_plugin_from_folder(&folder)
        .await
    {
        Ok(manifest) => manifest,
        Err(error) => {
            // A partially-written or invalid candidate must not tear down the
            // currently working runtime.
            emit(
                app,
                &package_id,
                PackageLifecyclePhase::Failed,
                100,
                "Package update could not be validated; the current runtime was kept",
                Some(error.clone()),
                false,
                core.plugin_manager()
                    .is_package_runtime_loaded(&package_id)
                    .await,
            );
            restore_package_authority(app, &package_id).await;
            return Err(error);
        }
    };
    let package_id = manifest.id.clone();
    let database = Database::new(db_pool);
    if let Some(receipt) = database.get_installed_package_release(&package_id).await?
        && receipt.source == "marketplace"
        && let Some(expected) = receipt.installed_content_sha256.as_deref()
    {
        let folder_for_hash = folder.clone();
        let actual =
            tokio::task::spawn_blocking(move || hash_directory(&folder_for_hash).map(hex::encode))
                .await
                .map_err(|error| format!("Package integrity hash task failed: {error}"))??;
        if !actual.eq_ignore_ascii_case(expected) {
            core.plugin_manager()
                .unload_package_runtime(&package_id)
                .await?;
            emit(
                app,
                &package_id,
                PackageLifecyclePhase::Failed,
                100,
                "Verified marketplace package was modified outside VortiDeck; repair or reinstall it",
                Some(
                    "Installed package content no longer matches its verified receipt".to_string(),
                ),
                false,
                false,
            );
            return Err(format!(
                "Marketplace package '{}' failed installed-content verification",
                package_id
            ));
        }
    }
    let grants = database.get_all_package_permission_grants().await?;
    let grant = matching_grant(&grants, &package_id);
    let consent_valid = grant_matches(grant, &manifest)?;
    let has_runtime = VortiDeckCore::plugin_folder_has_runtime(&folder)?;

    if !consent_valid {
        core.plugin_manager()
            .unload_package_runtime(&package_id)
            .await?;
        // Keep the durable record so the UI can request re-consent, but remove
        // active authority immediately. Otherwise a new frame or host call
        // could reuse the old in-memory grant while the update awaits review.
        core.plugin_manager()
            .get_permission_checker()
            .revoke_all_permissions(&package_id);
        if let Some(receipt) = database.get_installed_package_release(&package_id).await? {
            preserve_installed_receipt_with_state(&database, &receipt, "awaiting_review").await?;
        }
        emit(
            app,
            &package_id,
            PackageLifecyclePhase::AwaitingReview,
            100,
            if grant.is_some() {
                "Package access changed; review the update before it can run"
            } else {
                "Package discovered; review access before installation"
            },
            None,
            grant.is_some(),
            false,
        );
        return Ok(true);
    }

    // A runtime artifact can be removed while manifest.json remains. Treat
    // that as an unavailable runtime, not as a contribution-only update that
    // accidentally leaves the old code running in memory.
    if change.event_type == "removed" && !has_runtime {
        core.plugin_manager()
            .unload_package_runtime(&package_id)
            .await?;
        emit(
            app,
            &package_id,
            PackageLifecyclePhase::Unavailable,
            100,
            "Package runtime was removed; saved references remain unavailable",
            None,
            false,
            false,
        );
        return Ok(true);
    }

    let permissions = compile_manifest_permissions(&manifest);
    core.plugin_manager()
        .get_permission_checker()
        .grant_permissions(&manifest.id, permissions.clone());
    if !has_runtime {
        mark_installed_receipt_active(&database, &package_id).await?;
        restore_package_authority(app, &package_id).await;
        emit(
            app,
            &package_id,
            PackageLifecyclePhase::Active,
            100,
            "Package contributions updated",
            None,
            false,
            false,
        );
        return Ok(true);
    }

    // notify can emit several events for one completed write. Avoid repeatedly
    // stopping and recompiling an already-active, byte-identical package.
    if core
        .plugin_manager()
        .is_package_runtime_loaded(&package_id)
        .await
        && matches_last_known_good(&package_id, &folder).await?
    {
        restore_package_authority(app, &package_id).await;
        emit(
            app,
            &package_id,
            PackageLifecyclePhase::Active,
            100,
            "Package runtime is already current",
            None,
            false,
            true,
        );
        return Ok(false);
    }
    let allowed = core.plugin_manager().get_allowed_plugin_count().await as usize;
    if core.plugin_manager().get_loaded_package_count().await >= allowed {
        restore_package_authority(app, &package_id).await;
        emit(
            app,
            &package_id,
            PackageLifecyclePhase::GrantedInactive,
            100,
            "Package access is granted, but the runtime limit is reached",
            None,
            false,
            false,
        );
        return Ok(true);
    }

    emit(
        app,
        &package_id,
        PackageLifecyclePhase::Updating,
        35,
        "Stopping the previous runtime…",
        None,
        false,
        true,
    );
    core.plugin_manager()
        .unload_package_runtime(&package_id)
        .await?;
    emit(
        app,
        &package_id,
        PackageLifecyclePhase::Compiling,
        60,
        "Compiling and activating the updated runtime…",
        None,
        false,
        false,
    );
    match load_runtime_with_timeout(core, &folder).await {
        Ok(_) => {
            let backup_error = record_last_known_good(&package_id, &folder).await.err();
            if let Some(error) = &backup_error {
                log::warn!(
                    "Package '{}' activated, but its recovery snapshot failed: {}",
                    package_id,
                    error
                );
            }
            mark_installed_receipt_active(&database, &package_id).await?;
            restore_package_authority(app, &package_id).await;
            emit(
                app,
                &package_id,
                PackageLifecyclePhase::Active,
                100,
                if backup_error.is_some() {
                    "Package update activated, but its recovery snapshot failed"
                } else {
                    "Package update activated"
                },
                None,
                false,
                true,
            );
            Ok(true)
        }
        Err(update_error) => {
            match rollback_runtime(app, core, &package_id, Some(&permissions)).await {
                Ok(true) => {
                    restore_package_authority(app, &package_id).await;
                    Ok(true)
                }
                Ok(false) => {
                    emit(
                        app,
                        &package_id,
                        PackageLifecyclePhase::GrantedInactive,
                        100,
                        "Update failed and no previous runtime backup was available",
                        Some(update_error.clone()),
                        false,
                        false,
                    );
                    Err(update_error)
                }
                Err(rollback_error) => {
                    let error = format!("{update_error}; {rollback_error}");
                    emit(
                        app,
                        &package_id,
                        PackageLifecyclePhase::Failed,
                        100,
                        "Update and rollback both failed",
                        Some(error.clone()),
                        false,
                        false,
                    );
                    Err(error)
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct MarketplaceReleaseReceipt {
    pub release_id: String,
    pub version: String,
    pub artifact_sha256: String,
    pub marketplace_key_id: String,
}

/// Publish a verified marketplace candidate through the single lifecycle
/// authority. Download/verification code is deliberately unable to expose a
/// package to the watcher or runtime on its own.
pub async fn install_marketplace_candidate(
    app: &AppHandle,
    core: &VortiDeckCore,
    db_pool: &DbPool,
    candidate: &Path,
    package_id: &str,
    receipt: MarketplaceReleaseReceipt,
) -> Result<(), String> {
    if find_package_folder(package_id).await?.is_some() {
        return apply_marketplace_update(app, core, db_pool, candidate, package_id, receipt).await;
    }

    let _guard = lifecycle_lock().lock().await;
    let package_id = canonical_package_id(package_id).to_string();
    let manifest = core
        .plugin_manager()
        .validate_plugin_folder(candidate)
        .await?;
    if canonical_package_id(&manifest.id) != package_id || manifest.version != receipt.version {
        return Err("Verified install candidate identity changed before installation".to_string());
    }

    let destination = VortiDeckCore::get_default_plugins_directory().join(&manifest.id);
    if destination.exists() {
        return Err(format!(
            "Package '{}' appeared while installation was being prepared; refresh and retry",
            package_id
        ));
    }

    let database = Database::new(db_pool);
    let previous_receipt = database.get_installed_package_release(&package_id).await?;
    let candidate_for_hash = candidate.to_path_buf();
    let candidate_content_sha256 =
        tokio::task::spawn_blocking(move || hash_directory(&candidate_for_hash).map(hex::encode))
            .await
            .map_err(|error| format!("Package content hash task failed: {error}"))??;
    let now = chrono::Utc::now().to_rfc3339();
    database
        .save_installed_package_release(NewInstalledPackageRelease {
            package_id: &package_id,
            installed_version: &receipt.version,
            source: "marketplace",
            release_id: Some(&receipt.release_id),
            artifact_sha256: &receipt.artifact_sha256,
            marketplace_key_id: Some(&receipt.marketplace_key_id),
            installed_at: &now,
            updated_at: &now,
            auto_update: true,
            lifecycle_state: "installing",
            last_known_good_version: None,
            last_known_good_release_id: None,
            last_known_good_artifact_sha256: None,
            last_known_good_marketplace_key_id: None,
            installed_content_sha256: Some(&candidate_content_sha256),
        })
        .await?;

    let operation = async {
        fs::rename(candidate, &destination)
            .map_err(|error| format!("Failed to publish verified package: {error}"))?;
        core.plugin_manager()
            .discover_plugin_from_folder(&destination)
            .await
            .map_err(|error| format!("Marketplace package discovery failed: {error}"))?;
        database
            .save_installed_package_release(NewInstalledPackageRelease {
                package_id: &package_id,
                installed_version: &receipt.version,
                source: "marketplace",
                release_id: Some(&receipt.release_id),
                artifact_sha256: &receipt.artifact_sha256,
                marketplace_key_id: Some(&receipt.marketplace_key_id),
                installed_at: &now,
                updated_at: &now,
                auto_update: true,
                lifecycle_state: "awaiting_review",
                last_known_good_version: None,
                last_known_good_release_id: None,
                last_known_good_artifact_sha256: None,
                last_known_good_marketplace_key_id: None,
                installed_content_sha256: Some(&candidate_content_sha256),
            })
            .await?;
        Ok::<(), String>(())
    }
    .await;

    if let Err(error) = operation {
        core.plugin_manager()
            .remove_discovered_package(&package_id)
            .await;
        if destination.exists() {
            let _ = fs::remove_dir_all(&destination);
        }
        match previous_receipt.as_ref() {
            Some(previous) => restore_installed_release(&database, previous).await?,
            None => {
                database
                    .remove_installed_package_release(&package_id)
                    .await?
            }
        }
        return Err(error);
    }

    emit(
        app,
        &package_id,
        PackageLifecyclePhase::AwaitingReview,
        100,
        "Package verified; review requested access before activation",
        None,
        true,
        false,
    );
    Ok(())
}

async fn restore_marketplace_update(
    core: &VortiDeckCore,
    database: &Database,
    package_id: &str,
    current: &Path,
    previous: &Path,
    previous_receipt: Option<&InstalledPackageRelease>,
    runtime_was_active: bool,
) -> Result<(), String> {
    let mut failures = Vec::new();
    let _ = core
        .plugin_manager()
        .unload_package_runtime(package_id)
        .await;
    if current.exists() {
        if let Err(error) = fs::remove_dir_all(current) {
            failures.push(format!("failed to remove rejected update: {error}"));
        }
    }
    if previous.exists() {
        if let Err(error) = fs::rename(previous, current) {
            failures.push(format!("failed to restore previous package files: {error}"));
        }
    } else {
        failures.push("previous package staging is missing".to_string());
    }
    if current.exists() {
        if let Err(error) = core
            .plugin_manager()
            .discover_plugin_from_folder(current)
            .await
        {
            failures.push(format!("failed to rediscover previous package: {error}"));
        }
        if runtime_was_active {
            if let Err(error) = load_runtime_with_timeout(core, current).await {
                failures.push(format!("failed to restart previous runtime: {error}"));
            }
        }
    }
    match previous_receipt {
        Some(receipt) => {
            if let Err(error) = restore_installed_release(database, receipt).await {
                failures.push(format!("failed to restore installation receipt: {error}"));
            }
        }
        None => {
            if let Err(error) = database.remove_installed_package_release(package_id).await {
                failures.push(format!("failed to remove update receipt: {error}"));
            }
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

/// Atomically replace an existing marketplace package with a verified staged
/// candidate. This is the only marketplace update path: download code cannot
/// mutate the watched plugins directory or restart a runtime by itself.
pub async fn apply_marketplace_update(
    app: &AppHandle,
    core: &VortiDeckCore,
    db_pool: &DbPool,
    candidate: &Path,
    package_id: &str,
    receipt: MarketplaceReleaseReceipt,
) -> Result<(), String> {
    let _guard = lifecycle_lock().lock().await;
    let package_id = canonical_package_id(package_id).to_string();
    let current = find_package_folder(&package_id)
        .await?
        .ok_or_else(|| format!("Package '{package_id}' is not installed"))?;
    let candidate_manifest = core
        .plugin_manager()
        .validate_plugin_folder(candidate)
        .await?;
    if canonical_package_id(&candidate_manifest.id) != package_id
        || candidate_manifest.version != receipt.version
    {
        return Err("Verified update candidate identity changed before installation".to_string());
    }
    invalidate_package_authority(app, &package_id).await;

    let database = Database::new(db_pool);
    let previous_receipt = database.get_installed_package_release(&package_id).await?;
    let candidate_for_hash = candidate.to_path_buf();
    let candidate_content_sha256 =
        tokio::task::spawn_blocking(move || hash_directory(&candidate_for_hash).map(hex::encode))
            .await
            .map_err(|error| format!("Package content hash task failed: {error}"))??;
    let runtime_was_active = core
        .plugin_manager()
        .is_package_runtime_loaded(&package_id)
        .await;
    let previous_version = previous_receipt
        .as_ref()
        .map(|value| value.installed_version.clone())
        .or_else(|| {
            fs::read_to_string(current.join("manifest.json"))
                .ok()
                .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
                .and_then(|manifest| manifest.get("version")?.as_str().map(str::to_string))
        });

    emit(
        app,
        &package_id,
        PackageLifecyclePhase::Updating,
        78,
        "Snapshotting the current package before update…",
        None,
        false,
        runtime_was_active,
    );
    record_last_known_good(&package_id, &current).await?;
    core.plugin_manager()
        .unload_package_runtime(&package_id)
        .await?;

    let previous = candidate
        .parent()
        .ok_or_else(|| "Update staging directory has no parent".to_string())?
        .join(format!(
            "{}-previous-{}",
            safe_id(&package_id),
            uuid::Uuid::new_v4()
        ));
    fs::rename(&current, &previous)
        .map_err(|error| format!("Failed to stage current package for update: {error}"))?;
    if let Err(error) = fs::rename(candidate, &current) {
        let restore = fs::rename(&previous, &current).err();
        return Err(match restore {
            Some(restore) => format!(
                "Failed to publish package update: {error}; restoring package also failed: {restore}"
            ),
            None => format!("Failed to publish package update: {error}"),
        });
    }

    let operation = async {
        let manifest = core
            .plugin_manager()
            .discover_plugin_from_folder(&current)
            .await?;
        let grants = database.get_all_package_permission_grants().await?;
        let grant = matching_grant(&grants, &package_id);
        let consent_valid = grant_matches(grant, &manifest)?;
        let has_runtime = VortiDeckCore::plugin_folder_has_runtime(&current)?;
        let now = chrono::Utc::now().to_rfc3339();
        database
            .save_installed_package_release(NewInstalledPackageRelease {
                package_id: &package_id,
                installed_version: &receipt.version,
                source: "marketplace",
                release_id: Some(&receipt.release_id),
                artifact_sha256: &receipt.artifact_sha256,
                marketplace_key_id: Some(&receipt.marketplace_key_id),
                installed_at: previous_receipt
                    .as_ref()
                    .map(|value| value.installed_at.as_str())
                    .unwrap_or(&now),
                updated_at: &now,
                auto_update: previous_receipt
                    .as_ref()
                    .map(|value| value.auto_update)
                    .unwrap_or(true),
                lifecycle_state: if consent_valid {
                    "updating"
                } else {
                    "awaiting_review"
                },
                last_known_good_version: previous_version.as_deref(),
                last_known_good_release_id: previous_receipt
                    .as_ref()
                    .and_then(|value| value.release_id.as_deref()),
                last_known_good_artifact_sha256: previous_receipt
                    .as_ref()
                    .map(|value| value.artifact_sha256.as_str()),
                last_known_good_marketplace_key_id: previous_receipt
                    .as_ref()
                    .and_then(|value| value.marketplace_key_id.as_deref()),
                installed_content_sha256: Some(&candidate_content_sha256),
            })
            .await?;

        if !consent_valid {
            emit(
                app,
                &package_id,
                PackageLifecyclePhase::AwaitingReview,
                100,
                "Update verified; review its access before activation",
                None,
                grant.is_some(),
                false,
            );
            return Ok::<bool, String>(false);
        }

        core.plugin_manager()
            .get_permission_checker()
            .grant_permissions(&manifest.id, compile_manifest_permissions(&manifest));
        if has_runtime {
            emit(
                app,
                &package_id,
                PackageLifecyclePhase::Compiling,
                90,
                "Activating the verified update…",
                None,
                false,
                false,
            );
            load_runtime_with_timeout(core, &current).await?;
        }
        record_last_known_good(&package_id, &current).await?;
        mark_installed_receipt_active(&database, &package_id).await?;
        emit(
            app,
            &package_id,
            PackageLifecyclePhase::Active,
            100,
            "Package update installed and active",
            None,
            false,
            has_runtime,
        );
        Ok(true)
    }
    .await;

    match operation {
        Ok(authority_active) => {
            if let Err(error) = fs::remove_dir_all(&previous) {
                log::warn!(
                    "Package '{}' updated; deferred previous-folder cleanup for {}: {}",
                    package_id,
                    previous.display(),
                    error
                );
            }
            if authority_active {
                restore_package_authority(app, &package_id).await;
            }
            Ok(())
        }
        Err(error) => {
            let rollback = restore_marketplace_update(
                core,
                &database,
                &package_id,
                &current,
                &previous,
                previous_receipt.as_ref(),
                runtime_was_active,
            )
            .await;
            let error = match rollback {
                Ok(()) => {
                    let grants = database.get_all_package_permission_grants().await?;
                    if matching_grant(&grants, &package_id).is_some() {
                        restore_package_authority(app, &package_id).await;
                    }
                    error
                }
                Err(rollback_error) => format!("{error}; rollback failed: {rollback_error}"),
            };
            emit(
                app,
                &package_id,
                PackageLifecyclePhase::Failed,
                100,
                "Package update failed; the previous version was restored",
                Some(error.clone()),
                false,
                core.plugin_manager()
                    .is_package_runtime_loaded(&package_id)
                    .await,
            );
            Err(error)
        }
    }
}

/// Transactional user uninstall for every package kind. The folder is staged
/// out of the watched directory before runtime/grant removal and restored if a
/// later step fails.
pub async fn uninstall_package(
    app: &AppHandle,
    core: &VortiDeckCore,
    db_pool: &DbPool,
    package_id: &str,
) -> Result<(), String> {
    let _guard = lifecycle_lock().lock().await;
    let canonical = canonical_package_id(package_id);
    invalidate_package_authority(app, canonical).await;
    emit(
        app,
        canonical,
        PackageLifecyclePhase::Uninstalling,
        15,
        "Staging package removal…",
        None,
        false,
        core.plugin_manager()
            .is_package_runtime_loaded(package_id)
            .await,
    );

    let plugins_dir = VortiDeckCore::get_default_plugins_directory();
    let package_folder = fs::read_dir(&plugins_dir)
        .ok()
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .find(|folder| {
            fs::read_to_string(folder.join("manifest.json"))
                .ok()
                .and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok())
                .and_then(|manifest| manifest.get("id")?.as_str().map(str::to_string))
                .is_some_and(|id| canonical_package_id(&id) == canonical)
        });
    // Resolve every fallible database read before moving package files. From
    // this point onward, failures are handled by the compensation block.
    let database = Database::new(db_pool);
    let grants = database.get_all_package_permission_grants().await?;
    let matching_grants: Vec<_> = grants
        .into_iter()
        .filter(|grant| canonical_package_id(&grant.package_id) == canonical)
        .collect();
    let installed_release = database.get_installed_package_release(canonical).await?;
    let trash_root = plugins_dir
        .parent()
        .ok_or_else(|| "Plugin directory has no parent".to_string())?
        .join("plugin-uninstall-staging");
    fs::create_dir_all(&trash_root)
        .map_err(|error| format!("Failed to create uninstall staging: {error}"))?;
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let staged_folder = trash_root.join(format!("{}-{nonce}", safe_id(canonical)));

    if let Some(folder) = &package_folder {
        fs::rename(folder, &staged_folder)
            .map_err(|error| format!("Failed to stage package folder for removal: {error}"))?;
    }

    let operation = async {
        core.plugin_manager()
            .unload_package_runtime(canonical)
            .await?;
        core.plugin_manager()
            .remove_discovered_package(canonical)
            .await;
        let checker = core.plugin_manager().get_permission_checker();
        checker.revoke_all_permissions(canonical);
        checker.revoke_all_permissions(&format!("plugin.{canonical}"));
        let grant_ids = matching_grants
            .iter()
            .map(|grant| grant.package_id.clone())
            .collect::<Vec<_>>();
        database
            .remove_package_permission_grants(&grant_ids)
            .await?;
        database.remove_installed_package_release(canonical).await?;
        Ok::<(), String>(())
    }
    .await;

    if let Err(error) = operation {
        let grant_restore_error = database
            .restore_package_permission_grants(&matching_grants)
            .await
            .err();
        let receipt_restore_error = match &installed_release {
            Some(receipt) => restore_installed_release(&database, receipt).await.err(),
            None => None,
        };
        let mut runtime_restore_error = None;
        if let Some(original) = &package_folder {
            if staged_folder.exists() {
                if let Err(restore_error) = fs::rename(&staged_folder, original) {
                    runtime_restore_error =
                        Some(format!("package folder rollback failed: {restore_error}"));
                }
                let checker = core.plugin_manager().get_permission_checker();
                for grant in &matching_grants {
                    match serde_json::from_str(&grant.permissions_json) {
                        Ok(permissions) => {
                            checker.grant_permissions(&grant.package_id, permissions);
                        }
                        Err(parse_error) => {
                            runtime_restore_error =
                                Some(format!("permission rollback failed: {parse_error}"));
                        }
                    }
                }
                if original.exists() {
                    if let Err(restore_error) = load_runtime_with_timeout(core, original).await {
                        runtime_restore_error =
                            Some(format!("runtime rollback failed: {restore_error}"));
                    }
                }
            }
        }
        let rollback_complete = grant_restore_error.is_none()
            && receipt_restore_error.is_none()
            && runtime_restore_error.is_none();
        let error = match grant_restore_error {
            Some(restore_error) => format!("{error}; consent rollback failed: {restore_error}"),
            None => error,
        };
        let error = match runtime_restore_error {
            Some(restore_error) => format!("{error}; {restore_error}"),
            None => error,
        };
        let error = match receipt_restore_error {
            Some(restore_error) => {
                format!("{error}; installation receipt rollback failed: {restore_error}")
            }
            None => error,
        };
        emit(
            app,
            canonical,
            PackageLifecyclePhase::Failed,
            100,
            if rollback_complete {
                "Uninstall failed; package state was restored"
            } else {
                "Uninstall failed and rollback requires attention"
            },
            Some(error.clone()),
            false,
            core.plugin_manager()
                .is_package_runtime_loaded(canonical)
                .await,
        );
        if rollback_complete && !matching_grants.is_empty() {
            restore_package_authority(app, canonical).await;
        }
        return Err(error);
    }

    if staged_folder.exists() {
        if let Err(error) = fs::remove_dir_all(&staged_folder) {
            log::warn!(
                "Package '{}' was uninstalled; deferred staging cleanup for {}: {}",
                canonical,
                staged_folder.display(),
                error
            );
        }
    }
    let backup = backup_path(canonical)?;
    if backup.exists() {
        if let Err(error) = fs::remove_dir_all(&backup) {
            log::warn!(
                "Package '{}' was uninstalled; deferred backup cleanup for {}: {}",
                canonical,
                backup.display(),
                error
            );
        }
    }
    emit(
        app,
        canonical,
        PackageLifecyclePhase::Uninstalled,
        100,
        "Package uninstalled",
        None,
        false,
        false,
    );
    Ok(())
}

async fn restore_installed_release(
    database: &Database,
    receipt: &InstalledPackageRelease,
) -> Result<(), String> {
    database
        .save_installed_package_release(NewInstalledPackageRelease {
            package_id: &receipt.package_id,
            installed_version: &receipt.installed_version,
            source: &receipt.source,
            release_id: receipt.release_id.as_deref(),
            artifact_sha256: &receipt.artifact_sha256,
            marketplace_key_id: receipt.marketplace_key_id.as_deref(),
            installed_at: &receipt.installed_at,
            updated_at: &receipt.updated_at,
            auto_update: receipt.auto_update,
            lifecycle_state: &receipt.lifecycle_state,
            last_known_good_version: receipt.last_known_good_version.as_deref(),
            last_known_good_release_id: receipt.last_known_good_release_id.as_deref(),
            last_known_good_artifact_sha256: receipt.last_known_good_artifact_sha256.as_deref(),
            last_known_good_marketplace_key_id: receipt
                .last_known_good_marketplace_key_id
                .as_deref(),
            installed_content_sha256: receipt.installed_content_sha256.as_deref(),
        })
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use vortideck_core::db::models::PackagePermissionGrant;
    use vortideck_core::plugins::BundleManifest;

    fn manifest() -> BundleManifest {
        serde_json::from_value(serde_json::json!({
            "id": "sample",
            "name": "Sample",
            "version": "1.0.0",
            "author": "VortiDeck",
            "description": "Sample",
            "vortideck_version": ">=0.7.0",
            "schema_version": "1.0",
            "permissions": ["storage"]
        }))
        .expect("manifest")
    }

    #[test]
    fn canonical_runtime_prefix_matches_public_package_id() {
        assert_eq!(canonical_package_id("plugin.sample"), "sample");
        assert_eq!(canonical_package_id("sample"), "sample");
    }

    #[test]
    fn grant_validation_detects_permission_expansion() {
        let mut manifest = manifest();
        let permissions_json =
            serde_json::to_string(&compile_manifest_permissions(&manifest)).expect("permissions");
        let grant = PackagePermissionGrant {
            package_id: "sample".into(),
            manifest_json: serde_json::to_string(&manifest).expect("manifest json"),
            permissions_json,
            content_hash: manifest_consent_hash(&manifest).expect("hash"),
            trust: "community".into(),
            granted_at: "now".into(),
        };
        assert!(grant_matches(Some(&grant), &manifest).expect("valid grant"));
        manifest
            .permissions
            .push(vortideck_core::plugins::PluginPermission::Network);
        assert!(!grant_matches(Some(&grant), &manifest).expect("stale grant"));
    }

    #[test]
    fn directory_hash_is_stable_and_detects_artifact_changes() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("vortideck-package-hash-{nonce}"));
        let left = root.join("left");
        let right = root.join("right");
        fs::create_dir_all(left.join("web")).expect("left folder");
        fs::create_dir_all(right.join("web")).expect("right folder");
        fs::write(left.join("manifest.json"), "{}").expect("left manifest");
        fs::write(right.join("manifest.json"), "{}").expect("right manifest");
        fs::write(left.join("web/widget.js"), "safe").expect("left widget");
        fs::write(right.join("web/widget.js"), "safe").expect("right widget");
        assert_eq!(
            hash_directory(&left).unwrap(),
            hash_directory(&right).unwrap()
        );

        fs::write(right.join("web/widget.js"), "changed").expect("changed widget");
        assert_ne!(
            hash_directory(&left).unwrap(),
            hash_directory(&right).unwrap()
        );
        fs::remove_dir_all(root).expect("cleanup");
    }
}
