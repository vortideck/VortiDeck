use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use sysinfo::System;
use tauri::{AppHandle, Manager};
use uuid::Uuid;

// build.rs points this at the staged DLL, or at an empty placeholder when it is not staged.
static EMBEDDED_DLL: &[u8] = include_bytes!(env!("VORTIDECK_OBS_COMPANION_DLL"));
static EMBEDDED_MANIFEST: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/resources/obs-companion/manifest.json"
));

const RECEIPT_NAME: &str = "vortideck-companion.receipt.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObsArchitecture {
    X86_64,
    Arm64,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ObsInstallKind {
    Shared,
    Standard,
    Custom,
    Portable,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedCompanionManifest {
    pub schema_version: u32,
    pub id: String,
    pub version: String,
    pub protocol_version: u32,
    pub platform: String,
    pub architecture: ObsArchitecture,
    pub file_name: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsCompanionTarget {
    pub id: String,
    pub display_name: String,
    pub kind: ObsInstallKind,
    pub root_path: PathBuf,
    pub executable_path: Option<PathBuf>,
    pub architecture: ObsArchitecture,
    pub plugin_path: PathBuf,
    pub receipt_path: PathBuf,
    pub installed: bool,
    pub installed_version: Option<String>,
    pub installed_sha256: Option<String>,
    pub healthy: bool,
    pub status_detail: String,
    pub requires_elevation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsCompanionOverview {
    pub payload: EmbeddedCompanionManifest,
    pub obs_running: bool,
    pub targets: Vec<ObsCompanionTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsTargetRequest {
    pub kind: ObsInstallKind,
    pub root_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompanionReceipt {
    schema_version: u32,
    owner: String,
    target_kind: ObsInstallKind,
    obs_root: PathBuf,
    plugin_path: PathBuf,
    version: String,
    architecture: ObsArchitecture,
    sha256: String,
    installed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ElevatedRequest {
    action: String,
    target: ObsTargetRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct ManagedTargetLedger {
    schema_version: u32,
    targets: Vec<ObsTargetRequest>,
}

fn embedded_manifest() -> Result<EmbeddedCompanionManifest, String> {
    if EMBEDDED_DLL.is_empty() {
        return Err("This build does not include the OBS companion. Official VortiDeck releases include it; \
            for a source build, run scripts/stage-obs-companion.ps1 before building."
            .to_string());
    }
    let manifest: EmbeddedCompanionManifest =
        serde_json::from_str(EMBEDDED_MANIFEST).map_err(|error| error.to_string())?;
    let actual = sha256_bytes(EMBEDDED_DLL);
    if manifest.sha256.to_uppercase() != actual {
        return Err(format!(
            "Embedded OBS companion manifest hash mismatch: expected {}, got {}",
            manifest.sha256, actual
        ));
    }
    if manifest.architecture != pe_architecture(EMBEDDED_DLL) {
        return Err("Embedded OBS companion architecture does not match its manifest".to_string());
    }
    Ok(manifest)
}

fn sha256_bytes(bytes: &[u8]) -> String {
    format!("{:X}", Sha256::digest(bytes))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("Failed to read {}: {error}", path.display()))?;
    Ok(sha256_bytes(&bytes))
}

fn pe_architecture(bytes: &[u8]) -> ObsArchitecture {
    if bytes.len() < 0x40 || &bytes[0..2] != b"MZ" {
        return ObsArchitecture::Unknown;
    }
    let offset = u32::from_le_bytes(bytes[0x3c..0x40].try_into().unwrap()) as usize;
    if offset + 6 > bytes.len() || &bytes[offset..offset + 4] != b"PE\0\0" {
        return ObsArchitecture::Unknown;
    }
    match u16::from_le_bytes([bytes[offset + 4], bytes[offset + 5]]) {
        0x8664 => ObsArchitecture::X86_64,
        0xAA64 => ObsArchitecture::Arm64,
        _ => ObsArchitecture::Unknown,
    }
}

fn executable_architecture(path: &Path) -> ObsArchitecture {
    fs::read(path)
        .map(|bytes| pe_architecture(&bytes))
        .unwrap_or(ObsArchitecture::Unknown)
}

fn obs_is_running() -> bool {
    let system = System::new_all();
    system
        .processes()
        .values()
        .any(|process| process.name().eq_ignore_ascii_case("obs64.exe"))
}

fn strip_verbatim_prefix(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(stripped) = s.strip_prefix(r"\\?\") {
        PathBuf::from(stripped)
    } else {
        path
    }
}

fn running_obs_roots() -> Vec<PathBuf> {
    let system = System::new_all();
    system
        .processes()
        .values()
        .filter(|process| process.name().eq_ignore_ascii_case("obs64.exe"))
        .filter_map(|process| process.exe())
        .filter_map(|executable| {
            executable
                .parent()
                .and_then(Path::parent)
                .and_then(Path::parent)
                .map(|p| strip_verbatim_prefix(p.to_path_buf()))
        })
        .collect()
}

#[cfg(target_os = "windows")]
fn program_data_root() -> PathBuf {
    std::env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
}

#[cfg(not(target_os = "windows"))]
fn program_data_root() -> PathBuf {
    PathBuf::from("/var/lib")
}

fn shared_paths() -> (PathBuf, PathBuf) {
    let root = program_data_root()
        .join("obs-studio")
        .join("plugins")
        .join("vorti-obs-plugin");
    (
        root.join("bin").join("64bit").join("vorti-obs-plugin.dll"),
        root.join(RECEIPT_NAME),
    )
}

fn instance_paths(root: &Path) -> (PathBuf, PathBuf) {
    (
        root.join("obs-plugins")
            .join("64bit")
            .join("vorti-obs-plugin.dll"),
        root.join("data")
            .join("obs-plugins")
            .join("vorti-obs-plugin")
            .join(RECEIPT_NAME),
    )
}

fn obs_executable(root: &Path) -> PathBuf {
    root.join("bin").join("64bit").join("obs64.exe")
}

fn is_portable_root(root: &Path) -> bool {
    root.join("portable_mode").exists() || root.join("portable_mode.txt").exists()
}

fn normalize_existing_root(path: &Path) -> Result<PathBuf, String> {
    let mut root = strip_verbatim_prefix(path.to_path_buf());
    if root.is_file() {
        if root
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("obs64.exe"))
        {
            root = root
                .parent()
                .and_then(Path::parent)
                .and_then(Path::parent)
                .ok_or_else(|| {
                    "OBS executable does not have the expected bin/64bit layout".to_string()
                })?
                .to_path_buf();
        } else {
            return Err("Select the OBS root directory or obs64.exe".to_string());
        }
    }
    let root = fs::canonicalize(&root)
        .map_err(|error| format!("Cannot access OBS directory {}: {error}", root.display()))?;
    let root = strip_verbatim_prefix(root);
    if !obs_executable(&root).is_file() {
        return Err(format!(
            "{} does not contain bin\\64bit\\obs64.exe",
            root.display()
        ));
    }
    Ok(root)
}

fn target_id(kind: &ObsInstallKind, root: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!(
        "{:?}:{}",
        kind,
        root.to_string_lossy().to_lowercase()
    ));
    format!("{:x}", hasher.finalize())
}

fn read_receipt(path: &Path) -> Option<CompanionReceipt> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn describe_target(kind: ObsInstallKind, root: PathBuf) -> Result<ObsCompanionTarget, String> {
    let manifest = embedded_manifest()?;
    let executable = if kind == ObsInstallKind::Shared {
        None
    } else {
        Some(obs_executable(&root))
    };
    let architecture = executable
        .as_deref()
        .map(executable_architecture)
        .unwrap_or_else(|| manifest.architecture.clone());
    let (plugin_path, receipt_path) = if kind == ObsInstallKind::Shared {
        shared_paths()
    } else {
        instance_paths(&root)
    };
    let receipt = read_receipt(&receipt_path);
    let actual_hash = plugin_path
        .exists()
        .then(|| sha256_file(&plugin_path))
        .transpose()?;
    let installed = plugin_path.is_file();
    let owned = receipt
        .as_ref()
        .is_some_and(|item| item.owner == "VortiDeck" && item.plugin_path == plugin_path);
    let healthy = installed
        && owned
        && actual_hash
            .as_ref()
            .zip(receipt.as_ref())
            .is_some_and(|(hash, receipt)| hash.eq_ignore_ascii_case(&receipt.sha256));
    let status_detail = if healthy {
        "Installed and verified".to_string()
    } else if installed && !owned {
        "A plugin file exists but is not owned by VortiDeck".to_string()
    } else if installed {
        "Installed files differ from the ownership receipt; repair is available".to_string()
    } else if receipt.is_some() {
        "Receipt exists but the plugin DLL is missing; repair is available".to_string()
    } else {
        "Not installed".to_string()
    };
    let display_name = match kind {
        ObsInstallKind::Shared => "OBS Studio — shared system installation".to_string(),
        ObsInstallKind::Standard => format!("OBS Studio — {}", root.display()),
        ObsInstallKind::Custom => format!("Custom OBS — {}", root.display()),
        ObsInstallKind::Portable => format!("Portable OBS — {}", root.display()),
    };
    Ok(ObsCompanionTarget {
        id: target_id(&kind, &root),
        display_name,
        kind: kind.clone(),
        root_path: root,
        executable_path: executable,
        architecture,
        plugin_path,
        receipt_path,
        installed,
        installed_version: receipt.as_ref().map(|item| item.version.clone()),
        installed_sha256: actual_hash,
        healthy,
        status_detail,
        // Portable installations are normally user-writable. Standard/custom
        // roots and ProgramData are treated as privileged so the normal app
        // never receives broad write access to an OBS installation.
        requires_elevation: kind != ObsInstallKind::Portable,
    })
}

#[cfg(target_os = "windows")]
fn registry_obs_roots() -> Vec<PathBuf> {
    use winreg::enums::{
        HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY,
    };
    use winreg::RegKey;

    let mut roots = Vec::new();
    for hive in [
        RegKey::predef(HKEY_LOCAL_MACHINE),
        RegKey::predef(HKEY_CURRENT_USER),
    ] {
        for view in [KEY_READ | KEY_WOW64_64KEY, KEY_READ | KEY_WOW64_32KEY] {
            if let Ok(key) = hive.open_subkey_with_flags(r"SOFTWARE\OBS Studio", view) {
                if let Ok(path) = key.get_value::<String, _>("") {
                    roots.push(PathBuf::from(path));
                }
            }
            if let Ok(uninstall) = hive.open_subkey_with_flags(
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
                view,
            ) {
                for key_name in uninstall.enum_keys().flatten() {
                    let Ok(key) = uninstall.open_subkey_with_flags(&key_name, view) else {
                        continue;
                    };
                    let Ok(name) = key.get_value::<String, _>("DisplayName") else {
                        continue;
                    };
                    if !name.to_ascii_lowercase().starts_with("obs studio") {
                        continue;
                    }
                    if let Ok(path) = key.get_value::<String, _>("InstallLocation") {
                        if !path.trim().is_empty() {
                            roots.push(PathBuf::from(path));
                        }
                    }
                    if let Ok(icon) = key.get_value::<String, _>("DisplayIcon") {
                        let icon = icon.trim_matches('"').split(',').next().unwrap_or_default();
                        let icon_path = PathBuf::from(icon);
                        if let Some(root) = icon_path
                            .parent()
                            .and_then(Path::parent)
                            .and_then(Path::parent)
                        {
                            roots.push(root.to_path_buf());
                        }
                    }
                }
            }
        }
    }
    roots
}

#[cfg(not(target_os = "windows"))]
fn registry_obs_roots() -> Vec<PathBuf> {
    Vec::new()
}

fn common_obs_roots() -> Vec<PathBuf> {
    let mut roots = registry_obs_roots();
    roots.extend(running_obs_roots());
    for variable in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(base) = std::env::var_os(variable) {
            let base = PathBuf::from(base);
            roots.push(base.join("obs-studio"));
            roots.push(
                base.join("Steam")
                    .join("steamapps")
                    .join("common")
                    .join("OBS Studio"),
            );
        }
    }
    roots
}

fn ledger_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_local_data_dir()
        .map_err(|error| error.to_string())?
        .join("obs-companion")
        .join("targets.json"))
}

fn load_ledger(app: &AppHandle) -> ManagedTargetLedger {
    ledger_path(app)
        .ok()
        .and_then(|path| fs::read(path).ok())
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or(ManagedTargetLedger {
            schema_version: 1,
            targets: Vec::new(),
        })
}

fn remember_target(app: &AppHandle, request: &ObsTargetRequest) -> Result<(), String> {
    if request.kind == ObsInstallKind::Shared {
        return Ok(());
    }
    let mut ledger = load_ledger(app);
    ledger.schema_version = 1;
    if !ledger
        .targets
        .iter()
        .any(|target| target.kind == request.kind && target.root_path == request.root_path)
    {
        ledger.targets.push(request.clone());
    }
    let bytes = serde_json::to_vec_pretty(&ledger).map_err(|error| error.to_string())?;
    write_atomic(&ledger_path(app)?, &bytes)
}

fn forget_target(app: &AppHandle, request: &ObsTargetRequest) -> Result<(), String> {
    let mut ledger = load_ledger(app);
    ledger
        .targets
        .retain(|target| !(target.kind == request.kind && target.root_path == request.root_path));
    let bytes = serde_json::to_vec_pretty(&ledger).map_err(|error| error.to_string())?;
    write_atomic(&ledger_path(app)?, &bytes)
}

fn detected_targets(app: &AppHandle) -> Result<Vec<ObsCompanionTarget>, String> {
    let mut targets = Vec::new();
    let mut seen = HashSet::new();
    let shared_root = program_data_root().join("obs-studio");
    targets.push(describe_target(ObsInstallKind::Shared, shared_root)?);
    for candidate in common_obs_roots() {
        let Ok(root) = normalize_existing_root(&candidate) else {
            continue;
        };
        let key = root.to_string_lossy().to_lowercase();
        if !seen.insert(key) {
            continue;
        }
        let kind = if is_portable_root(&root) {
            ObsInstallKind::Portable
        } else {
            ObsInstallKind::Standard
        };
        targets.push(describe_target(kind, root)?);
    }
    for remembered in load_ledger(app).targets {
        let Ok(root) = normalize_existing_root(&remembered.root_path) else {
            continue;
        };
        let key = root.to_string_lossy().to_lowercase();
        if !seen.insert(key) {
            continue;
        }
        let kind = if is_portable_root(&root) {
            ObsInstallKind::Portable
        } else {
            ObsInstallKind::Custom
        };
        targets.push(describe_target(kind, root)?);
    }
    Ok(targets)
}

fn validated_target(request: &ObsTargetRequest) -> Result<ObsCompanionTarget, String> {
    if request.kind == ObsInstallKind::Shared {
        let expected = program_data_root().join("obs-studio");
        if request.root_path != expected {
            return Err("Shared OBS target path is not canonical".to_string());
        }
        return describe_target(ObsInstallKind::Shared, expected);
    }
    let root = normalize_existing_root(&request.root_path)?;
    let portable = is_portable_root(&root);
    if request.kind == ObsInstallKind::Portable && !portable {
        return Err("Selected OBS directory is not marked as portable".to_string());
    }
    if request.kind == ObsInstallKind::Standard && portable {
        return Err("Portable OBS must use the portable installation policy".to_string());
    }
    describe_target(request.kind.clone(), root)
}

fn canonical_request(target: &ObsCompanionTarget) -> ObsTargetRequest {
    ObsTargetRequest {
        kind: target.kind.clone(),
        root_path: target.root_path.clone(),
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Target has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    let staging = parent.join(format!(".vortideck-{}.tmp", Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)
        .map_err(|error| format!("Failed to stage {}: {error}", staging.display()))?;
    if let Err(error) = file.write_all(bytes).and_then(|_| file.sync_all()) {
        let _ = fs::remove_file(&staging);
        return Err(format!("Failed to write staged file: {error}"));
    }
    fs::rename(&staging, path).map_err(|error| {
        let _ = fs::remove_file(&staging);
        format!("Failed to commit {}: {error}", path.display())
    })
}

fn install_transaction(request: &ObsTargetRequest) -> Result<(), String> {
    if obs_is_running() {
        return Err("OBS Studio is running. Close OBS and try again.".to_string());
    }
    let target = validated_target(request)?;
    let manifest = embedded_manifest()?;
    if target.architecture != ObsArchitecture::Unknown
        && target.architecture != manifest.architecture
    {
        return Err(format!(
            "The bundled {:?} companion cannot be installed into {:?} OBS",
            manifest.architecture, target.architecture
        ));
    }
    if target.plugin_path.exists() && !target.receipt_path.exists() {
        return Err(format!(
            "Refusing to overwrite unowned plugin file {}",
            target.plugin_path.display()
        ));
    }

    let plugin_parent = target
        .plugin_path
        .parent()
        .ok_or_else(|| "Invalid plugin target".to_string())?;
    fs::create_dir_all(plugin_parent)
        .map_err(|error| format!("Failed to create {}: {error}", plugin_parent.display()))?;
    if let Some(parent) = target.receipt_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("Failed to create {}: {error}", parent.display()))?;
    }

    let transaction_id = Uuid::new_v4();
    let staged = plugin_parent.join(format!(".vortideck-{transaction_id}.new"));
    let backup = plugin_parent.join(format!(".vortideck-{transaction_id}.bak"));
    let mut staged_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staged)
        .map_err(|error| format!("Failed to create staged companion: {error}"))?;
    if let Err(error) = staged_file
        .write_all(EMBEDDED_DLL)
        .and_then(|_| staged_file.sync_all())
    {
        drop(staged_file);
        let _ = fs::remove_file(&staged);
        return Err(format!("Failed to stage companion: {error}"));
    }
    drop(staged_file);
    if sha256_file(&staged)? != manifest.sha256.to_uppercase() {
        let _ = fs::remove_file(&staged);
        return Err("Staged companion failed checksum verification".to_string());
    }

    let had_existing = target.plugin_path.exists();
    if had_existing {
        fs::rename(&target.plugin_path, &backup)
            .map_err(|error| format!("Failed to back up installed companion: {error}"))?;
    }
    if let Err(error) = fs::rename(&staged, &target.plugin_path) {
        if had_existing {
            let _ = fs::rename(&backup, &target.plugin_path);
        }
        let _ = fs::remove_file(&staged);
        return Err(format!("Failed to activate the new companion: {error}"));
    }

    let receipt = CompanionReceipt {
        schema_version: 1,
        owner: "VortiDeck".to_string(),
        target_kind: request.kind.clone(),
        obs_root: target.root_path.clone(),
        plugin_path: target.plugin_path.clone(),
        version: manifest.version,
        architecture: manifest.architecture,
        sha256: manifest.sha256.to_uppercase(),
        installed_at: Utc::now().to_rfc3339(),
    };
    let receipt_bytes = match serde_json::to_vec_pretty(&receipt) {
        Ok(bytes) => bytes,
        Err(error) => {
            let _ = fs::remove_file(&target.plugin_path);
            if had_existing {
                let _ = fs::rename(&backup, &target.plugin_path);
            }
            return Err(format!(
                "Companion receipt serialization failed; installation rolled back: {error}"
            ));
        }
    };
    if let Err(error) = write_atomic(&target.receipt_path, &receipt_bytes) {
        let _ = fs::remove_file(&target.plugin_path);
        if had_existing {
            let _ = fs::rename(&backup, &target.plugin_path);
        }
        return Err(format!(
            "Companion receipt failed; installation rolled back: {error}"
        ));
    }
    if had_existing {
        let _ = fs::remove_file(&backup);
    }
    Ok(())
}

fn remove_transaction(request: &ObsTargetRequest) -> Result<(), String> {
    if obs_is_running() {
        return Err("OBS Studio is running. Close OBS and try again.".to_string());
    }
    let target = validated_target(request)?;
    let receipt = read_receipt(&target.receipt_path).ok_or_else(|| {
        "No valid VortiDeck ownership receipt exists; refusing to remove files".to_string()
    })?;
    if receipt.owner != "VortiDeck" || receipt.plugin_path != target.plugin_path {
        return Err("Ownership receipt does not match the requested plugin target".to_string());
    }
    if target.plugin_path.exists() {
        let actual = sha256_file(&target.plugin_path)?;
        if !actual.eq_ignore_ascii_case(&receipt.sha256) {
            return Err(
                "Installed DLL was modified outside VortiDeck; refusing automatic removal"
                    .to_string(),
            );
        }
        let removed = target
            .plugin_path
            .with_file_name(format!(".vortideck-remove-{}.tmp", Uuid::new_v4()));
        let staged_receipt = target
            .receipt_path
            .with_file_name(format!(".vortideck-remove-{}.receipt", Uuid::new_v4()));
        fs::rename(&target.plugin_path, &removed)
            .map_err(|error| format!("Failed to stage companion removal: {error}"))?;
        if let Err(error) = fs::rename(&target.receipt_path, &staged_receipt) {
            let _ = fs::rename(&removed, &target.plugin_path);
            return Err(format!(
                "Failed to stage the ownership receipt; removal rolled back: {error}"
            ));
        }
        if let Err(error) = fs::remove_file(&removed) {
            let receipt_restored = fs::rename(&staged_receipt, &target.receipt_path).is_ok();
            let plugin_restored = fs::rename(&removed, &target.plugin_path).is_ok();
            return Err(format!(
                "Failed to finish companion removal; rollback receipt={receipt_restored}, plugin={plugin_restored}: {error}"
            ));
        }
        // The canonical DLL and receipt paths are now both absent, so removal is
        // committed. A failure to delete the staged receipt is only recoverable
        // cleanup and must not recreate a receipt for a plugin that is gone.
        let _ = fs::remove_file(&staged_receipt);
    } else {
        fs::remove_file(&target.receipt_path)
            .map_err(|error| format!("Failed to remove stale receipt: {error}"))?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn run_elevated(request: &ElevatedRequest) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let executable = std::env::current_exe().map_err(|error| error.to_string())?;
    let payload =
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(request).map_err(|error| error.to_string())?);
    let exe_escaped = executable.to_string_lossy().replace('\'', "''");
    let script = format!(
        "$p = Start-Process -FilePath '{exe_escaped}' -Verb RunAs -Wait -PassThru -WindowStyle Hidden -ArgumentList @('--vortideck-obs-companion-helper', '{payload}'); exit $p.ExitCode"
    );
    let mut cmd = Command::new("powershell.exe");
    cmd.creation_flags(CREATE_NO_WINDOW);
    let status = cmd
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-Command",
            &script,
        ])
        .status()
        .map_err(|error| format!("Failed to request elevation: {error}"))?;
    if !status.success() {
        return Err(format!(
            "Elevated companion operation failed or was cancelled ({status})"
        ));
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn run_elevated(_request: &ElevatedRequest) -> Result<(), String> {
    Err("Elevated OBS companion operations are currently implemented only on Windows".to_string())
}

fn perform_action(action: &str, target: &ObsTargetRequest, elevate: bool) -> Result<(), String> {
    if elevate {
        return run_elevated(&ElevatedRequest {
            action: action.to_string(),
            target: target.clone(),
        });
    }
    match action {
        "install" | "repair" => install_transaction(target),
        "remove" => remove_transaction(target),
        _ => Err("Unsupported OBS companion action".to_string()),
    }
}

pub fn run_elevated_helper_from_args() -> Option<i32> {
    let mut arguments = std::env::args();
    let _ = arguments.next();
    if arguments.next().as_deref() != Some("--vortideck-obs-companion-helper") {
        return None;
    }
    let result = arguments
        .next()
        .ok_or_else(|| "Missing elevated helper payload".to_string())
        .and_then(|payload| {
            URL_SAFE_NO_PAD
                .decode(payload)
                .map_err(|error| error.to_string())
        })
        .and_then(|bytes| {
            serde_json::from_slice::<ElevatedRequest>(&bytes).map_err(|error| error.to_string())
        })
        .and_then(|request| perform_action(&request.action, &request.target, false));
    if let Err(error) = result {
        let _ = writeln!(io::stderr(), "OBS companion helper failed: {error}");
        return Some(1);
    }
    Some(0)
}

#[tauri::command]
pub async fn get_obs_companion_overview(app: AppHandle) -> Result<ObsCompanionOverview, String> {
    Ok(ObsCompanionOverview {
        payload: embedded_manifest()?,
        obs_running: obs_is_running(),
        targets: detected_targets(&app)?,
    })
}

#[tauri::command]
pub async fn inspect_obs_companion_target(path: String) -> Result<ObsCompanionTarget, String> {
    let root = normalize_existing_root(Path::new(&path))?;
    let kind = if is_portable_root(&root) {
        ObsInstallKind::Portable
    } else {
        ObsInstallKind::Custom
    };
    describe_target(kind, root)
}

#[tauri::command]
pub async fn install_obs_companion(
    app: AppHandle,
    target: ObsTargetRequest,
) -> Result<ObsCompanionTarget, String> {
    let validated = validated_target(&target)?;
    perform_action("install", &target, validated.requires_elevation)?;
    let canonical = canonical_request(&validated);
    remember_target(&app, &canonical)?;
    validated_target(&canonical)
}

#[tauri::command]
pub async fn repair_obs_companion(
    app: AppHandle,
    target: ObsTargetRequest,
) -> Result<ObsCompanionTarget, String> {
    let validated = validated_target(&target)?;
    perform_action("repair", &target, validated.requires_elevation)?;
    let canonical = canonical_request(&validated);
    remember_target(&app, &canonical)?;
    validated_target(&canonical)
}

#[tauri::command]
pub async fn remove_obs_companion(target: ObsTargetRequest) -> Result<ObsCompanionTarget, String> {
    let validated = validated_target(&target)?;
    perform_action("remove", &target, validated.requires_elevation)?;
    validated_target(&target)
}

#[tauri::command]
pub async fn forget_obs_companion_target(
    app: AppHandle,
    target: ObsTargetRequest,
) -> Result<(), String> {
    let validated = validated_target(&target)?;
    if validated.installed || validated.receipt_path.exists() {
        return Err("Remove the managed companion before forgetting this target".to_string());
    }
    forget_target(&app, &canonical_request(&validated))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn fake_pe(machine: u16) -> Vec<u8> {
        let mut bytes = vec![0u8; 0x90];
        bytes[0..2].copy_from_slice(b"MZ");
        bytes[0x3c..0x40].copy_from_slice(&(0x80u32).to_le_bytes());
        bytes[0x80..0x84].copy_from_slice(b"PE\0\0");
        bytes[0x84..0x86].copy_from_slice(&machine.to_le_bytes());
        bytes
    }

    fn portable_request(temp: &TempDir) -> ObsTargetRequest {
        let root = temp.path();
        fs::create_dir_all(root.join("bin/64bit")).unwrap();
        fs::write(root.join("bin/64bit/obs64.exe"), fake_pe(0x8664)).unwrap();
        fs::write(root.join("portable_mode.txt"), []).unwrap();
        ObsTargetRequest {
            kind: ObsInstallKind::Portable,
            root_path: root.to_path_buf(),
        }
    }

    #[test]
    fn detects_pe_architectures() {
        assert_eq!(pe_architecture(&fake_pe(0x8664)), ObsArchitecture::X86_64);
        assert_eq!(pe_architecture(&fake_pe(0xAA64)), ObsArchitecture::Arm64);
        assert_eq!(pe_architecture(b"not a PE"), ObsArchitecture::Unknown);
    }

    #[test]
    fn installs_repairs_and_removes_with_receipt() {
        let temp = TempDir::new().unwrap();
        let request = portable_request(&temp);
        install_transaction(&request).unwrap();
        let target = validated_target(&request).unwrap();
        assert!(target.healthy);
        assert_eq!(fs::read(&target.plugin_path).unwrap(), EMBEDDED_DLL);

        fs::write(&target.plugin_path, b"damaged").unwrap();
        assert!(!validated_target(&request).unwrap().healthy);
        install_transaction(&request).unwrap();
        assert!(validated_target(&request).unwrap().healthy);

        remove_transaction(&request).unwrap();
        let target = validated_target(&request).unwrap();
        assert!(!target.installed);
        assert!(!target.receipt_path.exists());
    }

    #[test]
    fn refuses_to_overwrite_or_remove_unowned_files() {
        let temp = TempDir::new().unwrap();
        let request = portable_request(&temp);
        let target = validated_target(&request).unwrap();
        fs::create_dir_all(target.plugin_path.parent().unwrap()).unwrap();
        fs::write(&target.plugin_path, b"foreign").unwrap();
        assert!(install_transaction(&request)
            .unwrap_err()
            .contains("unowned"));
        assert!(remove_transaction(&request)
            .unwrap_err()
            .contains("ownership receipt"));
        assert_eq!(fs::read(&target.plugin_path).unwrap(), b"foreign");
    }

    #[test]
    fn rejects_architecture_mismatch_and_fake_portable_root() {
        let temp = TempDir::new().unwrap();
        let request = portable_request(&temp);
        fs::write(temp.path().join("bin/64bit/obs64.exe"), fake_pe(0xAA64)).unwrap();
        assert!(install_transaction(&request)
            .unwrap_err()
            .contains("cannot be installed"));

        fs::remove_file(temp.path().join("portable_mode.txt")).unwrap();
        assert!(validated_target(&request)
            .unwrap_err()
            .contains("not marked as portable"));
    }
}
