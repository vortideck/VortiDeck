# Installer and Multiplatform Release Architecture

Status date: 2026-08-28

## Decision

Windows keeps MSI/WiX as the canonical application installer. NSIS is intentionally not produced because VortiDeck's firewall rule, Explorer integration, upgrade identity, and application lifecycle policy are expressed as transactional WiX components. OBS companion lifecycle is now deliberately owned by VortiDeck rather than WiX.

The shared Tauri configuration now contains only common application resources. Tauri's automatically merged platform files select the package format and platform resources:

| Platform | Configuration | Application output | OBS companion output |
| --- | --- | --- | --- |
| Windows | `tauri.windows.conf.json` | MSI | Versioned x64 DLL embedded in VortiDeck and transactionally installed by the in-app companion manager |
| macOS | `tauri.macos.conf.json` | Signed/notarized app and DMG | Separately signed/notarized `.plugin` bundle |
| Linux | `tauri.linux.conf.json` | AppImage, deb, rpm, and Flatpak | Separately signed `.so` package |

The application and OBS companion are separate release units on macOS and Linux. The OBS source is maintained in the sibling `OBS-Custom-Websocket-Plugin` repository and now has its own `native-companion-release.yml` workflow for a signed/notarized universal macOS `.plugin` archive and signed Linux `.deb`/`.so` archive. The main application must not pretend a Windows DLL is portable.

## Windows ownership model

| Concern | Owner | Behavior |
| --- | --- | --- |
| Main executable and common resources | Tauri MSI | Installed under Program Files and repaired/uninstalled by MSI |
| OBS companion payload | VortiDeck executable | Known-compatible DLL and manifest are embedded at compile time; payload hash and PE architecture are verified before use |
| OBS installation lifecycle | In-app companion manager | Detects targets, requires OBS to be closed, stages and verifies the DLL, atomically replaces it, writes an ownership receipt, and rolls back failures |
| Protected OBS targets | Narrow elevated VortiDeck helper mode | UAC elevation performs only validated companion install/repair/remove operations; the normal GUI remains unelevated |
| Shell DLL and icon | Tauri MSI resources | Exactly one copy under `resources\shell-ext` |
| Modern Windows 11 shell registration | VortiDeck runtime | Per-user, versioned manifest staged under writable LocalAppData |
| Legacy Explorer verbs | MSI registry components | Files, directories, and directory backgrounds; removed transactionally |
| Sparse-package cleanup | Checked uninstall action | Runs for the uninstalling user before MSI removes the external files |
| Firewall access | WiX firewall extension | One private-profile, local-subnet, program-scoped rule |
| WebView2 prerequisite | Tauri embedded bootstrapper mode | Bootstrapper is carried inside the MSI instead of downloaded by a long WiX custom-action command; the bootstrapper obtains the current runtime when needed |

MSI no longer writes into ProgramData or an OBS directory and no longer contains `ObsCompanionFeature`, `INSTALL_OBS_COMPANION`, or an OBS-running custom action. Existing MSI-owned companion files are removed transactionally when an older MSI is major-upgraded. The companion is managed from **Settings → Connection → OBS Companion**, not the marketplace Plugins page, because it is a native OBS bridge rather than an uninstallable marketplace/runtime package.

## OBS installation modes and architectures

| OBS installation | Current behavior | Long-term behavior |
| --- | --- | --- |
| Standard Windows x64 | Detects registry/common locations and offers the shared ProgramData target or detected instance target; protected paths use the narrow UAC helper | Add live plugin-load confirmation after OBS restart |
| Custom non-portable Windows path | Discovers registry, common, and running-process paths or accepts a selected root; validates `bin\64bit\obs64.exe`, detects PE architecture, installs through the same transaction, and remembers/forgets the target | Add richer stale-registry diagnostics |
| Portable Windows x64 | User selects the portable root; VortiDeck requires `portable_mode`/`portable_mode.txt`, installs into `obs-plugins\64bit`, stores the ownership receipt under `data\obs-plugins\vorti-obs-plugin`, and remembers the target | Add a separately downloadable portable ZIP as a manual recovery option |
| Native Windows ARM64 | Not supported by the current x64 DLL | Build and sign a separate ARM64 DLL, publish it as an architecture-specific artifact, detect the OBS process architecture, and install only the matching DLL; never load x64 into native ARM64 OBS |
| macOS/Linux | Companion is a separate native release unit | Install the signed/notarized `.plugin` or signed `.so` package using the platform-specific workflow |

The manager never recursively searches disks or silently copies into every OBS-looking directory. It combines registry/common-path discovery, explicit custom selection, portable markers, PE architecture inspection, confirmation, persistent target memory, and a receipt adjacent to the managed plugin data. It refuses to overwrite foreign plugin files and refuses to remove a DLL that no longer matches its ownership receipt.

## Signing and release gates

Windows release signing is implemented by `src-tauri/scripts/sign-windows.ps1` and the release-only Tauri configuration. The workflow signs and timestamps:

- the Explorer shell-extension DLL;
- the VortiDeck executable through Tauri's `signCommand`;
- the final MSI through the same `signCommand`.

Required production secrets:

- `WINDOWS_CODESIGN_CERTIFICATE_BASE64`
- `WINDOWS_CODESIGN_CERTIFICATE_PASSWORD`
- existing Tauri updater private-key secrets

Set repository variable `REQUIRE_WINDOWS_CODE_SIGNING=true` to make an unsigned Windows release impossible. Local development remains usable without a certificate.

The OBS source repository publishes an architecture-specific Windows DLL, SHA-256 sidecar, and generated compatibility/provenance manifest. Authenticode is optional during development and can later be enforced with `REQUIRE_OBS_COMPANION_AUTHENTICODE=true`. Hash, GitHub asset digest, version, source commit, declared architecture, and compatibility validation remain mandatory even for explicitly unsigned development artifacts. VortiDeck releases never re-sign the verified DLL.

The normal release chain is:

1. Develop and test the companion in the OBS repository. Ordinary builds do not change its semantic version. For local manager testing, `scripts/stage-local-companion.ps1` regenerates VortiDeck's embedded development payload and hash without publishing anything.
2. When the change is ready for users, run `scripts/release.ps1 patch`, `minor`, or `major`. The helper updates the canonical `buildspec.json`, commits only that version change, creates the matching tag, and pushes both.
3. The tag triggers **Native OBS Companion Release**. The workflow creates a draft release at that exact commit, builds the DLL, signs it when configured, generates its checksum and source-commit manifest, and publishes only after every enabled platform succeeds.
4. Create the VortiDeck desktop version/tag normally. Its release workflow queries the latest stable companion release, downloads the x64 DLL, checksum, and manifest, verifies the GitHub digest, checksum, manifest version, VortiDeck minimum version, declared architecture, and optional Authenticode signature, then stages the exact unchanged DLL as an application build resource. Rust embeds that verified payload in the VortiDeck executable; WiX never owns or installs the standalone DLL. If that newest companion requires a newer VortiDeck version, the release fails visibly instead of silently choosing an older binary.
5. The desktop release uploads `obs-companion-provenance.json` beside the MSI, recording the resolved companion tag, commit, checksum, signer, and compatibility data.

A manually dispatched desktop release has an optional `obs_companion_tag` rollback input. Normal releases leave it blank and select the newest stable compatible companion automatically. Local MSI builds may still use the checked-in staging DLL, but production releases cannot fall back to it.

The cross-platform workflow is opt-in so it cannot break the established Windows release:

- `ENABLE_MACOS_RELEASE=true` enables macOS arm64 and Intel packages and requires a Developer ID identity plus notarization credentials.
- `ENABLE_LINUX_RELEASE=true` enables AppImage/deb/rpm/Flatpak packages and requires the Linux GPG key.

Linux provides an embedded AppImage signature, a native RPM signature, Tauri updater signatures, SHA-256 checksums, and detached GPG signatures for all four package formats. Debian repository metadata should also be signed when a package repository is introduced.

## Tests and acceptance gates

The following checks now exist:

| Check | Script/workflow | Coverage |
| --- | --- | --- |
| Source policy | `scripts/test-windows-installer.ps1` | MSI-only target, pinned upgrade code, no OBS MSI feature, embedded payload/receipt/rollback implementation, firewall scope, platform isolation |
| Compiled MSI database | `scripts/test-windows-msi-contents.ps1` | No standalone OBS DLL/feature/close action, single shell DLL/icon, runtime resource hierarchy, uninstall action, embedded WebView bootstrapper |
| Companion transactions | Rust tests in `obs_companion_commands.rs` | PE architecture, portable validation, install/repair/remove, ownership receipts, foreign-file refusal, modified-file preservation, architecture mismatch |
| Disposable lifecycle | `scripts/test-windows-msi-lifecycle.ps1` | Clean install, major upgrade cleanup of the legacy MSI-owned OBS DLL, destructive application repair, uninstall, optional signature enforcement |
| Pull request policy | `installer-validation.yml` | Runs non-destructive source checks |
| Release acceptance | `installer-lifecycle.yml` | Downloads SHA-256-pinned current/previous MSIs and runs compiled-content plus lifecycle checks on a disposable Windows runner |

Real OBS process-lock, UAC cancellation, portable-drive reconnect, and post-restart plugin-load confirmation remain end-to-end tests. The manager refuses operations while `obs64.exe` is running rather than terminating it.

## Validation performed locally

- Production Nuxt generation completed.
- The native Explorer extension built in release mode and was restaged.
- The full VortiDeck Rust release application built successfully on Windows.
- WiX 3.14.1 compiled every custom fragment and linked the MSI.
- The MSI embeds Microsoft's WebView2 bootstrapper, removing the download-command custom action and its WiX ICE03 warning without carrying the much larger offline runtime.
- A fresh post-migration `VortiDeck_0.6.9_x64_en-US.msi` was linked locally and passed the compiled-content test, including the stronger assertions that no standalone OBS DLL, OBS MSI feature, or OBS-running WiX action remains.
- The app-owned companion transaction suite passes all install, repair, removal, ownership, portable, and architecture tests. The checkout now embeds the hosted `v1.0.1` payload.
- PowerShell test scripts parse successfully.
- JSON, WiX XML, AppStream XML, workflow YAML, and Flatpak template syntax were checked.
- Live CDP inspection confirmed Settings → Connection renders the OBS Companion card, bundled version/architecture, ProgramData target, UAC disclosure, and install/custom-portable actions. The inspection exposed unresolved generic icon warnings; those icons were replaced with compiled project imports and production Nuxt generation passed afterward.

The local MSI is unsigned because production signing and Tauri updater private keys are correctly absent from the workstation environment. The disposable install/upgrade/repair/uninstall workflow has been implemented but must run against a signed release artifact before enabling production signing enforcement.

## Optional distribution hardening and external validation

These are credentials, additional platform work, or real-environment acceptance tests. They are not required for unsigned Windows development releases and are not missing Windows installer architecture:

1. Configure the Windows signing provider for both repositories and set `REQUIRE_WINDOWS_CODE_SIGNING=true` in the application repository. The current workflows accept a PFX; a newly issued hardware-token or cloud-HSM certificate needs its provider-specific signing integration.
2. The automated release path has published unsigned development releases
   through `v1.0.7`. The current source tag resolves to commit `2eae6fc`, and
   the VortiDeck staging verifier records the release asset digest, SHA-256,
   architecture, compatibility and unsigned state. Configure signing later
   before requiring Authenticode.
3. Run the MSI lifecycle workflow with the current signed MSI and one prior signed MSI.
4. Configure Apple Developer ID/notarization secrets, then enable and run the macOS matrix.
5. Configure the Linux GPG key, then enable and run Linux packaging plus Flatpak smoke tests.
6. Expand the conservative `release-compatibility.json` only after testing more OBS versions.
7. Run actual clean-machine smoke tests on each supported OS and actual OBS versions before advertising full multiplatform support.
8. Add portable relocation diagnostics, multiple-instance/stale-registry tests, a native ARM64 artifact, and live post-restart plugin-load confirmation.

Flatpak is a real package target, but VortiDeck's broad system-automation features require a separate sandbox capability review. Packaging success alone does not prove that every host-control feature is available inside Flatpak; unsupported operations must be disabled or implemented through portals/helpers before that package is called feature-equivalent.
