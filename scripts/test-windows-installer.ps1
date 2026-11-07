$ErrorActionPreference = 'Stop'

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$tauriRoot = Join-Path $repositoryRoot 'apps\tauri-windows\src-tauri'

function Assert-InstallerPolicy {
    param([bool] $Condition, [string] $Message)
    if (-not $Condition) { throw "Installer policy failed: $Message" }
}

$config = Get-Content -Raw -LiteralPath (Join-Path $tauriRoot 'tauri.windows.conf.json') | ConvertFrom-Json
$targets = @($config.bundle.targets)
Assert-InstallerPolicy ($targets.Count -eq 1 -and $targets[0] -eq 'msi') 'Windows must have one canonical MSI target.'
Assert-InstallerPolicy ($config.bundle.windows.webviewInstallMode.type -eq 'embedBootstrapper') 'Windows must embed the WebView2 bootstrapper instead of relying on a downloaded bootstrapper command.'
Assert-InstallerPolicy ($config.bundle.windows.wix.upgradeCode -eq '45733d75-581e-5e1e-bc63-5e9b937adb6b') 'The existing MSI upgrade code must remain pinned.'
Assert-InstallerPolicy (-not $config.bundle.windows.wix.featureRefs) 'OBS companion lifecycle must not be owned by an MSI feature.'

$macConfig = Get-Content -Raw -LiteralPath (Join-Path $tauriRoot 'tauri.macos.conf.json') | ConvertFrom-Json
Assert-InstallerPolicy (@($macConfig.bundle.targets).Count -eq 2 -and @($macConfig.bundle.targets) -contains 'app' -and @($macConfig.bundle.targets) -contains 'dmg') 'macOS must produce an app bundle and DMG.'
Assert-InstallerPolicy (-not ($macConfig.bundle.resources.PSObject.Properties.Name -match '\.dll$')) 'macOS resources must not inherit Windows DLLs.'

$linuxConfig = Get-Content -Raw -LiteralPath (Join-Path $tauriRoot 'tauri.linux.conf.json') | ConvertFrom-Json
$linuxTargets = @($linuxConfig.bundle.targets)
Assert-InstallerPolicy ($linuxTargets.Count -eq 3 -and $linuxTargets -contains 'appimage' -and $linuxTargets -contains 'deb' -and $linuxTargets -contains 'rpm') 'Linux must produce AppImage, deb, and rpm packages.'
Assert-InstallerPolicy (-not ($linuxConfig.bundle.resources.PSObject.Properties.Name -match '\.dll$')) 'Linux resources must not inherit Windows DLLs.'

$fragmentDirectory = Join-Path $tauriRoot 'windows\fragments'
$fragments = Get-ChildItem -LiteralPath $fragmentDirectory -Filter '*.wxs'
foreach ($fragment in $fragments) {
    $null = [xml](Get-Content -Raw -LiteralPath $fragment.FullName)
    $contents = Get-Content -Raw -LiteralPath $fragment.FullName
    Assert-InstallerPolicy ($contents -notmatch '(?i)Source="[A-Z]:\\') "$($fragment.Name) contains a machine-specific source path."
    Assert-InstallerPolicy ($contents -notmatch 'Return="ignore"') "$($fragment.Name) silently ignores a custom-action failure."
}

Assert-InstallerPolicy (-not (Test-Path -LiteralPath (Join-Path $fragmentDirectory 'obs_plugin.wxs'))) 'The obsolete MSI-owned OBS companion fragment must not return.'

$firewall = Get-Content -Raw -LiteralPath (Join-Path $fragmentDirectory 'firewall_exceptions.wxs')
Assert-InstallerPolicy ($firewall -match 'Profile="private"') 'Firewall access must be private-profile only.'
Assert-InstallerPolicy ($firewall -match 'Scope="localSubnet"') 'Firewall access must be local-subnet only.'
Assert-InstallerPolicy ($firewall -match 'IgnoreFailure="no"') 'Firewall installation failures must be visible and transactional.'
Assert-InstallerPolicy (-not ($firewall -match 'Program=' -and $firewall -match 'Protocol=')) 'A WiX program firewall exception must not also specify Protocol.'

$context = Get-Content -Raw -LiteralPath (Join-Path $fragmentDirectory 'context_menu.wxs')
Assert-InstallerPolicy ($context -notmatch 'Id="RegisterSparsePackage"') 'MSI must not compete with runtime per-user sparse-package registration.'
Assert-InstallerPolicy ($context -notmatch 'Id="ShellExtComRegistration"') 'MSI must not duplicate the sparse package COM registration.'
Assert-InstallerPolicy ($context -notmatch 'Id="SparsePackageRegistration"') 'MSI must not install a false per-user sparse-package marker.'
Assert-InstallerPolicy ($context -match 'Return="check"') 'Sparse-package uninstall cleanup must report failure.'
Assert-InstallerPolicy ($context -notmatch 'ExecutionPolicy Bypass') 'Installer custom actions must not bypass PowerShell policy.'
Assert-InstallerPolicy ($context -notmatch 'ShellExtDllComponent|SparsePackageComponent') 'Shell resources must have one owner through Tauri resource bundling.'

$requiredInputs = @(
    'resources\plugins\vorti-obs-plugin.dll',
    'resources\obs-companion\manifest.json',
    'resources\shell-ext\vortideck-shell-ext.dll',
    'resources\shell-ext\icon.png',
    'icons\128x128.png'
)
foreach ($relativePath in $requiredInputs) {
    Assert-InstallerPolicy (Test-Path -LiteralPath (Join-Path $tauriRoot $relativePath)) "Missing installer input: $relativePath"
}

# Release workflows live only in the private repository; the public repository leaves them out.
$releaseWorkflowPath = Join-Path $repositoryRoot '.github\workflows\desktop-release.yml'
if (Test-Path -LiteralPath $releaseWorkflowPath) {
    $workflow = Get-Content -Raw -LiteralPath $releaseWorkflowPath
    Assert-InstallerPolicy ($workflow -notmatch 'F:/vd-target') 'Release workflow must not depend on the F: drive.'
    Assert-InstallerPolicy ($workflow -match 'sign-windows.ps1') 'Release workflow must support Authenticode signing.'
    Assert-InstallerPolicy ($workflow -match '--bundles msi') 'Release workflow must publish the canonical MSI only.'
    Assert-InstallerPolicy ($workflow -match 'stage-obs-companion.ps1') 'Release workflow must resolve and verify a published OBS companion.'
    Assert-InstallerPolicy ($workflow -match 'obs-companion-provenance.json') 'Release workflow must publish the resolved companion provenance.'
    Assert-InstallerPolicy ($workflow -notmatch 'OBS_COMPANION_WINDOWS_SHA256|REQUIRE_OBS_COMPANION_PROVENANCE') 'Companion selection must not require manually synchronized repository variables.'
    Assert-InstallerPolicy ($workflow -notmatch 'sign-windows.ps1[^\r\n]+vorti-obs-plugin') 'The app release must not mutate the checksum-verified companion by signing it again.'
    Assert-InstallerPolicy ($workflow -match 'releaseDraft:\s*true') 'The release must remain a draft until clean-machine MSI lifecycle acceptance passes.'
    Assert-InstallerPolicy ($workflow -match 'windows-lifecycle-acceptance:' -and $workflow -match 'test-windows-msi-lifecycle\.ps1') 'The release must exercise clean install, upgrade, repair, and uninstall before publication.'
    Assert-InstallerPolicy ($workflow -match 'publish-release:' -and $workflow -match 'needs:\s*\[release, windows-lifecycle-acceptance\]') 'Publication must depend on successful MSI lifecycle acceptance.'
} else {
    Write-Host 'Skipping release workflow checks: desktop-release.yml is not in this repository.'
}
Assert-InstallerPolicy (Test-Path -LiteralPath (Join-Path $repositoryRoot 'scripts\stage-obs-companion.ps1')) 'Companion staging and provenance script is missing.'
$companionCommands = Get-Content -Raw -LiteralPath (Join-Path $tauriRoot 'src\commands\obs_companion_commands.rs')
Assert-InstallerPolicy ($companionCommands -match 'include_bytes!' -and $companionCommands -match 'vorti-obs-plugin.dll') 'The app must embed its offline companion payload.'
Assert-InstallerPolicy ($companionCommands -match 'CompanionReceipt' -and $companionCommands -match 'rolled back') 'The app-owned companion lifecycle must retain ownership receipts and rollback.'

$crossPlatformWorkflowPath = Join-Path $repositoryRoot '.github\workflows\desktop-cross-platform-release.yml'
if (Test-Path -LiteralPath $crossPlatformWorkflowPath) {
    $crossPlatformWorkflow = Get-Content -Raw -LiteralPath $crossPlatformWorkflowPath
    Assert-InstallerPolicy ($crossPlatformWorkflow -match "ENABLE_MACOS_RELEASE == 'true'") 'macOS release activation must remain explicit.'
    Assert-InstallerPolicy ($crossPlatformWorkflow -match "ENABLE_LINUX_RELEASE == 'true'") 'Linux release activation must remain explicit.'
    Assert-InstallerPolicy ($crossPlatformWorkflow -match '--bundles app,dmg') 'macOS release must publish app and DMG packages.'
    Assert-InstallerPolicy ($crossPlatformWorkflow -match '--bundles appimage,deb,rpm') 'Linux release must publish AppImage, deb, and rpm packages.'
} else {
    Write-Host 'Skipping release workflow checks: desktop-cross-platform-release.yml is not in this repository.'
}
Assert-InstallerPolicy (Test-Path -LiteralPath (Join-Path $repositoryRoot 'packaging\linux\com.vortideck.desktop.yml.in')) 'Flatpak manifest template is missing.'
Assert-InstallerPolicy (Test-Path -LiteralPath (Join-Path $repositoryRoot 'scripts\build-flatpak.sh')) 'Flatpak build script is missing.'

Write-Host 'Windows installer policy validation passed.' -ForegroundColor Green
