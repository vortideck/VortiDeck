use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::time::Duration;
use tauri::{AppHandle, State};
use vortideck_core::db::db::DbPool;
use vortideck_core::db::models::InstalledPackageRelease;
use vortideck_core::db::Database;
use vortideck_core::plugins::{canonical_package_id, PackageInfo};
use vortideck_core::services::auth::TokenStorage;
use vortideck_core::VortiDeckCore;

const MAX_ARTIFACT_BYTES: usize = 50 * 1024 * 1024;
const PINNED_KEY_B64: Option<&str> = option_env!("VORTIDECK_MARKETPLACE_ROOT_PUBLIC_KEY_B64");
const PINNED_KEY_ID: Option<&str> = option_env!("VORTIDECK_MARKETPLACE_ROOT_KEY_ID");
// The backup key's private half is kept offline. If the primary signing key is
// lost or exposed, the backend switches to the backup and installed apps keep
// verifying marketplace releases without an app update.
const PINNED_BACKUP_KEY_B64: Option<&str> =
    option_env!("VORTIDECK_MARKETPLACE_BACKUP_PUBLIC_KEY_B64");
const PINNED_BACKUP_KEY_ID: Option<&str> = option_env!("VORTIDECK_MARKETPLACE_BACKUP_KEY_ID");
const DEFAULT_PRIMARY_KEY_ID: &str = "marketplace-root-v1";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketplaceReleaseSummary {
    pub release_id: String,
    pub version: String,
    pub min_host_version: String,
    pub max_host_version: String,
    pub artifact_sha256: String,
    pub artifact_size: u64,
    #[serde(default)]
    pub permissions: serde_json::Value,
    pub published_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MarketplacePackage {
    pub package_id: String,
    pub slug: String,
    pub name: String,
    pub summary: String,
    pub description: String,
    pub category: String,
    pub author: String,
    pub trust: String,
    pub is_free: bool,
    pub icon_url: String,
    pub support_url: String,
    pub website_url: String,
    pub rating: Option<f64>,
    pub reviews: u64,
    pub downloads: u64,
    pub latest_release: Option<MarketplaceReleaseSummary>,
    pub updated_at: String,
}

#[derive(Deserialize)]
struct PaginatedPackages {
    results: Vec<MarketplacePackage>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PackageListResponse {
    Plain(Vec<MarketplacePackage>),
    Paginated(PaginatedPackages),
}

#[derive(Debug, Deserialize)]
struct DownloadTicket {
    release_id: String,
    package_id: String,
    version: String,
    artifact_url: String,
    artifact_sha256: String,
    artifact_size: u64,
    attestation: serde_json::Value,
    marketplace_signature: String,
    marketplace_key_id: String,
}

#[derive(Serialize)]
pub struct PluginLibraryItem {
    pub package_id: String,
    pub catalog: Option<MarketplacePackage>,
    pub local: Option<PackageInfo>,
    pub installation: Option<InstalledPackageRelease>,
}

#[derive(Serialize)]
pub struct PluginLibraryResponse {
    pub items: Vec<PluginLibraryItem>,
    pub catalog_error: Option<String>,
}

fn api_base_url() -> String {
    std::env::var("API_URL")
        .unwrap_or_else(|_| "https://backend.vortideck.com".to_string())
        .trim_end_matches('/')
        .to_string()
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|error| format!("Failed to create marketplace HTTP client: {error}"))
}

async fn response_error(response: reqwest::Response, operation: &str) -> String {
    let status = response.status();
    let message = response
        .text()
        .await
        .unwrap_or_default()
        .chars()
        .take(1000)
        .collect::<String>();
    format!("Marketplace {operation} failed ({status}): {message}")
}

async fn fetch_catalog() -> Result<Vec<MarketplacePackage>, String> {
    let response = client()?
        .get(format!("{}/api/v1/marketplace/packages/", api_base_url()))
        .query(&[("host_version", env!("CARGO_PKG_VERSION"))])
        .send()
        .await
        .map_err(|error| format!("Marketplace catalog request failed: {error}"))?;
    if !response.status().is_success() {
        return Err(response_error(response, "catalog request").await);
    }
    let response = response
        .json::<PackageListResponse>()
        .await
        .map_err(|error| format!("Marketplace catalog response is invalid: {error}"))?;
    Ok(match response {
        PackageListResponse::Plain(packages) => packages,
        PackageListResponse::Paginated(page) => page.results,
    })
}

#[tauri::command]
pub async fn list_plugin_library(
    core: State<'_, VortiDeckCore>,
    db_pool: State<'_, DbPool>,
) -> Result<PluginLibraryResponse, String> {
    let local =
        crate::commands::widget_commands::list_packages_impl(core.inner(), db_pool.inner()).await?;
    let installations = Database::new(db_pool.inner())
        .get_all_installed_package_releases()
        .await?;
    let (catalog, catalog_error) = match fetch_catalog().await {
        Ok(packages) => (packages, None),
        Err(error) => (Vec::new(), Some(error)),
    };
    let mut items = BTreeMap::<String, PluginLibraryItem>::new();
    for package in catalog {
        let id = canonical_package_id(&package.package_id).to_string();
        items.insert(
            id.clone(),
            PluginLibraryItem {
                package_id: id,
                catalog: Some(package),
                local: None,
                installation: None,
            },
        );
    }
    for package in local {
        let id = canonical_package_id(&package.id).to_string();
        items
            .entry(id.clone())
            .or_insert_with(|| PluginLibraryItem {
                package_id: id,
                catalog: None,
                local: None,
                installation: None,
            })
            .local = Some(package);
    }
    for installation in installations {
        let id = canonical_package_id(&installation.package_id).to_string();
        items
            .entry(id.clone())
            .or_insert_with(|| PluginLibraryItem {
                package_id: id,
                catalog: None,
                local: None,
                installation: None,
            })
            .installation = Some(installation);
    }
    Ok(PluginLibraryResponse {
        items: items.into_values().collect(),
        catalog_error,
    })
}

fn canonicalize_json(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => serde_json::Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), canonicalize_json(value)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(canonicalize_json).collect())
        }
        value => value.clone(),
    }
}

/// A compiled-in value, or in debug builds the runtime environment variable.
fn pinned_value(compiled: Option<&str>, variable: &str) -> Option<String> {
    compiled
        .map(str::to_string)
        .or_else(|| {
            cfg!(debug_assertions)
                .then(|| std::env::var(variable).ok())
                .flatten()
        })
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn decode_public_key(label: &str, encoded: &str) -> Result<VerifyingKey, String> {
    let raw = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| format!("Pinned marketplace {label} key is malformed"))?;
    let raw: [u8; 32] = raw
        .try_into()
        .map_err(|_| format!("Pinned marketplace {label} key must be 32 bytes"))?;
    VerifyingKey::from_bytes(&raw).map_err(|_| format!("Pinned marketplace {label} key is invalid"))
}

/// Builds the trusted key set: the primary key is required, the backup key is optional
/// but must carry its own key id.
fn trusted_keys_from(
    primary: Option<String>,
    primary_id: Option<String>,
    backup: Option<String>,
    backup_id: Option<String>,
) -> Result<Vec<(String, VerifyingKey)>, String> {
    let primary = primary.ok_or_else(|| {
        "Marketplace root key is not pinned in this build; remote installation is disabled"
            .to_string()
    })?;
    let primary_id = primary_id.unwrap_or_else(|| DEFAULT_PRIMARY_KEY_ID.to_string());
    let mut keys = vec![(primary_id, decode_public_key("root", &primary)?)];
    if let Some(backup) = backup {
        let backup_id = backup_id.ok_or_else(|| {
            "The marketplace backup key is pinned without VORTIDECK_MARKETPLACE_BACKUP_KEY_ID"
                .to_string()
        })?;
        if backup_id == keys[0].0 {
            return Err(
                "The marketplace backup key id must differ from the root key id".to_string(),
            );
        }
        let backup = decode_public_key("backup", &backup)?;
        if backup == keys[0].1 {
            return Err("The marketplace backup key must differ from the root key".to_string());
        }
        keys.push((backup_id, backup));
    }
    Ok(keys)
}

fn trusted_keys() -> Result<Vec<(String, VerifyingKey)>, String> {
    trusted_keys_from(
        pinned_value(PINNED_KEY_B64, "VORTIDECK_MARKETPLACE_ROOT_PUBLIC_KEY_B64"),
        pinned_value(PINNED_KEY_ID, "VORTIDECK_MARKETPLACE_ROOT_KEY_ID"),
        pinned_value(
            PINNED_BACKUP_KEY_B64,
            "VORTIDECK_MARKETPLACE_BACKUP_PUBLIC_KEY_B64",
        ),
        pinned_value(PINNED_BACKUP_KEY_ID, "VORTIDECK_MARKETPLACE_BACKUP_KEY_ID"),
    )
}

fn verify_compatibility(
    attestation: &serde_json::Map<String, serde_json::Value>,
) -> Result<(), String> {
    let host = semver::Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("Desktop version is invalid: {error}"))?;
    for (field, minimum) in [("min_host_version", true), ("max_host_version", false)] {
        let Some(bound) = attestation.get(field).and_then(|value| value.as_str()) else {
            continue;
        };
        if bound.is_empty() {
            continue;
        }
        let bound = semver::Version::parse(bound)
            .map_err(|_| format!("Marketplace attestation has invalid {field}"))?;
        if (minimum && host < bound) || (!minimum && host > bound) {
            return Err(format!(
                "Marketplace release is incompatible with VortiDeck {host}"
            ));
        }
    }
    Ok(())
}

fn verify_ticket(ticket: &DownloadTicket, artifact: &[u8]) -> Result<(), String> {
    verify_ticket_with_keys(ticket, artifact, &trusted_keys()?)
}

fn verify_ticket_with_keys(
    ticket: &DownloadTicket,
    artifact: &[u8],
    trusted_keys: &[(String, VerifyingKey)],
) -> Result<(), String> {
    if artifact.is_empty() || artifact.len() > MAX_ARTIFACT_BYTES {
        return Err("Marketplace artifact size is outside the supported range".to_string());
    }
    if artifact.len() as u64 != ticket.artifact_size {
        return Err("Marketplace artifact size does not match the release ticket".to_string());
    }
    if hex::encode(Sha256::digest(artifact)) != ticket.artifact_sha256 {
        return Err("Marketplace artifact SHA-256 verification failed".to_string());
    }
    let attestation = ticket
        .attestation
        .as_object()
        .ok_or_else(|| "Marketplace attestation is not an object".to_string())?;
    for (field, expected) in [
        ("package_id", ticket.package_id.as_str()),
        ("release_id", ticket.release_id.as_str()),
        ("version", ticket.version.as_str()),
        ("artifact_sha256", ticket.artifact_sha256.as_str()),
    ] {
        if attestation.get(field).and_then(|value| value.as_str()) != Some(expected) {
            return Err(format!(
                "Marketplace attestation field '{field}' does not match"
            ));
        }
    }
    if attestation
        .get("artifact_size")
        .and_then(|value| value.as_u64())
        != Some(ticket.artifact_size)
    {
        return Err("Marketplace attestation artifact_size does not match".to_string());
    }
    verify_compatibility(attestation)?;
    let key = trusted_keys
        .iter()
        .find(|(key_id, _)| *key_id == ticket.marketplace_key_id)
        .map(|(_, key)| key)
        .ok_or_else(|| {
            format!(
                "Marketplace attestation uses unexpected key '{}'",
                ticket.marketplace_key_id
            )
        })?;
    let signature = base64::engine::general_purpose::STANDARD
        .decode(&ticket.marketplace_signature)
        .map_err(|_| "Marketplace signature is malformed".to_string())?;
    let signature = Signature::from_slice(&signature)
        .map_err(|_| "Marketplace signature has an invalid length".to_string())?;
    let message = serde_json::to_vec(&canonicalize_json(&ticket.attestation))
        .map_err(|error| format!("Failed to canonicalize marketplace attestation: {error}"))?;
    key.verify(&message, &signature)
        .map_err(|_| "Marketplace attestation signature verification failed".to_string())
}

async fn download_ticket(release_id: &str) -> Result<(DownloadTicket, String), String> {
    let token = TokenStorage::get_access_token()
        .map_err(|_| "Sign in before downloading marketplace packages".to_string())?;
    let response = client()?
        .post(format!(
            "{}/api/v1/marketplace/releases/{release_id}/download/",
            api_base_url()
        ))
        .bearer_auth(&token)
        .json(&serde_json::json!({
            "client_version": env!("CARGO_PKG_VERSION"),
            "platform": std::env::consts::OS,
        }))
        .send()
        .await
        .map_err(|error| format!("Marketplace authorization failed: {error}"))?;
    if !response.status().is_success() {
        return Err(response_error(response, "download authorization").await);
    }
    let ticket = response
        .json::<DownloadTicket>()
        .await
        .map_err(|error| format!("Marketplace download ticket is invalid: {error}"))?;
    Ok((ticket, token))
}

async fn download_artifact(ticket: &DownloadTicket, token: &str) -> Result<Vec<u8>, String> {
    let mut request = client()?.get(&ticket.artifact_url);
    if ticket.artifact_url.starts_with(&api_base_url()) {
        request = request.bearer_auth(token);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("Marketplace artifact download failed: {error}"))?;
    if !response.status().is_success() {
        return Err(response_error(response, "artifact download").await);
    }
    if response
        .content_length()
        .is_some_and(|size| size > MAX_ARTIFACT_BYTES as u64)
    {
        return Err("Marketplace artifact exceeds the 50 MiB limit".to_string());
    }
    let bytes = response
        .bytes()
        .await
        .map_err(|error| format!("Failed to read marketplace artifact: {error}"))?;
    if bytes.len() > MAX_ARTIFACT_BYTES {
        return Err("Marketplace artifact exceeds the 50 MiB limit".to_string());
    }
    Ok(bytes.to_vec())
}

#[tauri::command]
pub async fn install_marketplace_release(
    release_id: String,
    package_id: String,
    app: AppHandle,
    core: State<'_, VortiDeckCore>,
    db_pool: State<'_, DbPool>,
) -> Result<String, String> {
    crate::package_lifecycle::report(
        &app,
        &package_id,
        crate::package_lifecycle::PackageLifecyclePhase::Inspecting,
        10,
        "Authorizing marketplace download…",
        None,
        false,
        false,
    );
    let operation = async {
        let (ticket, token) = download_ticket(&release_id).await?;
        if canonical_package_id(&ticket.package_id) != canonical_package_id(&package_id) {
            return Err("Marketplace release resolved to a different package".to_string());
        }
        crate::package_lifecycle::report(
            &app,
            &package_id,
            crate::package_lifecycle::PackageLifecyclePhase::Downloading,
            30,
            "Downloading private marketplace artifact…",
            None,
            false,
            false,
        );
        let artifact = download_artifact(&ticket, &token).await?;
        crate::package_lifecycle::report(
            &app,
            &package_id,
            crate::package_lifecycle::PackageLifecyclePhase::Verifying,
            55,
            "Verifying artifact hash and marketplace signature…",
            None,
            false,
            false,
        );
        verify_ticket(&ticket, &artifact)?;

        let temporary = std::env::temp_dir().join(format!(
            "vortideck-marketplace-{}.vdext",
            uuid::Uuid::new_v4()
        ));
        tokio::fs::write(&temporary, &artifact)
            .await
            .map_err(|error| format!("Failed to stage marketplace artifact: {error}"))?;
        crate::package_lifecycle::report(
            &app,
            &package_id,
            crate::package_lifecycle::PackageLifecyclePhase::Staging,
            75,
            "Publishing verified package into local staging…",
            None,
            false,
            false,
        );
        let folder = core
            .plugin_manager()
            .stage_verified_package_archive(&temporary, &ticket.package_id, &ticket.version)
            .await;
        let _ = tokio::fs::remove_file(&temporary).await;
        let folder = folder?;
        let install = crate::package_lifecycle::install_marketplace_candidate(
            &app,
            core.inner(),
            db_pool.inner(),
            &folder,
            &ticket.package_id,
            crate::package_lifecycle::MarketplaceReleaseReceipt {
                release_id: ticket.release_id.clone(),
                version: ticket.version.clone(),
                artifact_sha256: ticket.artifact_sha256.clone(),
                marketplace_key_id: ticket.marketplace_key_id.clone(),
            },
        )
        .await;
        if install.is_err() && folder.exists() {
            let _ = tokio::fs::remove_dir_all(&folder).await;
        }
        install?;
        Ok(ticket.package_id)
    }
    .await;
    if let Err(error) = &operation {
        crate::package_lifecycle::report(
            &app,
            &package_id,
            crate::package_lifecycle::PackageLifecyclePhase::Failed,
            100,
            "Marketplace installation failed",
            Some(error.clone()),
            false,
            false,
        );
    }
    operation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_json_sorts_nested_keys() {
        let value = serde_json::json!({"z": 1, "a": {"y": 2, "b": 3}});
        assert_eq!(
            serde_json::to_string(&canonicalize_json(&value)).unwrap(),
            r#"{"a":{"b":3,"y":2},"z":1}"#
        );
    }

    use ed25519_dalek::{Signer, SigningKey};

    const ARTIFACT: &[u8] = b"package bytes";

    fn signing_key(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    fn encoded_public(key: &SigningKey) -> String {
        base64::engine::general_purpose::STANDARD.encode(key.verifying_key().to_bytes())
    }

    fn signed_ticket(key_id: &str, key: &SigningKey) -> DownloadTicket {
        let sha256 = hex::encode(Sha256::digest(ARTIFACT));
        let attestation = serde_json::json!({
            "package_id": "example.package",
            "release_id": "release-1",
            "version": "1.0.0",
            "artifact_sha256": sha256,
            "artifact_size": ARTIFACT.len(),
        });
        let message = serde_json::to_vec(&canonicalize_json(&attestation)).unwrap();
        DownloadTicket {
            release_id: "release-1".to_string(),
            package_id: "example.package".to_string(),
            version: "1.0.0".to_string(),
            artifact_url: String::new(),
            artifact_sha256: sha256,
            artifact_size: ARTIFACT.len() as u64,
            attestation,
            marketplace_signature: base64::engine::general_purpose::STANDARD
                .encode(key.sign(&message).to_bytes()),
            marketplace_key_id: key_id.to_string(),
        }
    }

    fn primary_and_backup() -> Vec<(String, VerifyingKey)> {
        trusted_keys_from(
            Some(encoded_public(&signing_key(1))),
            Some("marketplace-root-v1".to_string()),
            Some(encoded_public(&signing_key(2))),
            Some("marketplace-backup-v1".to_string()),
        )
        .unwrap()
    }

    #[test]
    fn releases_signed_by_the_primary_or_backup_key_verify() {
        let keys = primary_and_backup();
        let primary = signed_ticket("marketplace-root-v1", &signing_key(1));
        let backup = signed_ticket("marketplace-backup-v1", &signing_key(2));
        assert_eq!(verify_ticket_with_keys(&primary, ARTIFACT, &keys), Ok(()));
        assert_eq!(verify_ticket_with_keys(&backup, ARTIFACT, &keys), Ok(()));
    }

    #[test]
    fn unknown_key_ids_and_forged_signatures_are_rejected() {
        let keys = primary_and_backup();
        let unknown = signed_ticket("marketplace-root-v2", &signing_key(3));
        assert!(verify_ticket_with_keys(&unknown, ARTIFACT, &keys)
            .unwrap_err()
            .contains("unexpected key"));
        // Signed by an untrusted key while claiming the backup key id.
        let forged = signed_ticket("marketplace-backup-v1", &signing_key(3));
        assert!(verify_ticket_with_keys(&forged, ARTIFACT, &keys)
            .unwrap_err()
            .contains("signature verification failed"));
    }

    #[test]
    fn primary_only_builds_reject_backup_signed_releases() {
        let keys =
            trusted_keys_from(Some(encoded_public(&signing_key(1))), None, None, None).unwrap();
        assert_eq!(keys[0].0, DEFAULT_PRIMARY_KEY_ID);
        let backup = signed_ticket("marketplace-backup-v1", &signing_key(2));
        assert!(verify_ticket_with_keys(&backup, ARTIFACT, &keys).is_err());
    }

    #[test]
    fn invalid_key_configurations_fail_closed() {
        let primary = Some(encoded_public(&signing_key(1)));
        let backup = Some(encoded_public(&signing_key(2)));
        let root_id = Some("marketplace-root-v1".to_string());
        assert!(trusted_keys_from(None, None, backup.clone(), Some("b".to_string())).is_err());
        assert!(trusted_keys_from(primary.clone(), root_id.clone(), backup.clone(), None).is_err());
        assert!(
            trusted_keys_from(primary.clone(), root_id.clone(), backup, root_id.clone()).is_err()
        );
        assert!(trusted_keys_from(
            primary.clone(),
            root_id,
            primary,
            Some("marketplace-backup-v1".to_string())
        )
        .is_err());
    }
}
