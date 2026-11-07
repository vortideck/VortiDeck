$ErrorActionPreference = 'Stop'

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$tauriRoot = Join-Path $repositoryRoot 'apps\tauri-windows\src-tauri'
$coreRoot = Join-Path $repositoryRoot 'apps\tauri-windows\libs\vortideck-core'

function Get-PackageVersion {
    param([string] $ManifestPath)

    $manifest = Get-Content -Raw -LiteralPath $ManifestPath
    $packageSection = [regex]::Match($manifest, '(?ms)^\[package\]\s*(.*?)(?=^\[|\z)')
    if (-not $packageSection.Success) { throw "Missing [package] section in $ManifestPath" }

    $version = [regex]::Match($packageSection.Groups[1].Value, '(?m)^version\s*=\s*"([^"]+)"\s*$')
    if (-not $version.Success) { throw "Missing package version in $ManifestPath" }
    return $version.Groups[1].Value
}

$tauriVersion = (Get-Content -Raw -LiteralPath (Join-Path $tauriRoot 'tauri.conf.json') | ConvertFrom-Json).version
$desktopVersion = Get-PackageVersion (Join-Path $tauriRoot 'Cargo.toml')
$coreVersion = Get-PackageVersion (Join-Path $coreRoot 'Cargo.toml')

if ($tauriVersion -notmatch '^\d+\.\d+\.\d+([+-][0-9A-Za-z.-]+)?$') {
    throw "Tauri version '$tauriVersion' is not a valid release version."
}
if ($desktopVersion -ne $tauriVersion) {
    throw "Desktop Cargo version '$desktopVersion' does not match Tauri version '$tauriVersion'."
}
if ($coreVersion -ne $tauriVersion) {
    throw "Core Cargo version '$coreVersion' does not match Tauri version '$tauriVersion'."
}

$settings = Get-Content -Raw -LiteralPath (Join-Path $repositoryRoot 'layers\desktop\components\Modal\SettingsModal.vue')
if ($settings -match 'appVersion\s*\|\|\s*[''"]\d+\.\d+\.\d+') {
    throw 'Settings must read the packaged version instead of duplicating a release-number fallback.'
}

Write-Host "Release version alignment passed: $tauriVersion" -ForegroundColor Green
