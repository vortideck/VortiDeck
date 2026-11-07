param(
    [Parameter(Mandatory = $true)]
    [string] $Destination,

    [Parameter(Mandatory = $true)]
    [string] $ProvenancePath,

    [string] $EmbeddedManifestDestination,

    [string] $ReleaseTag,
    [string] $Repository = 'alexiokay/OBS-Custom-Websocket-Plugin',
    [string] $VortiDeckConfig = 'apps/tauri-windows/src-tauri/tauri.conf.json',
    [string] $GitHubToken = $env:GH_TOKEN,
    [switch] $RequireAuthenticode
)

$ErrorActionPreference = 'Stop'
$headers = @{
    Accept = 'application/vnd.github+json'
    'X-GitHub-Api-Version' = '2022-11-28'
    'User-Agent' = 'VortiDeck-release'
}
if ($GitHubToken) { $headers.Authorization = "Bearer $GitHubToken" }

$releaseEndpoint = if ($ReleaseTag) {
    "https://api.github.com/repos/$Repository/releases/tags/$([Uri]::EscapeDataString($ReleaseTag))"
} else {
    "https://api.github.com/repos/$Repository/releases/latest"
}
$release = Invoke-RestMethod -Uri $releaseEndpoint -Headers $headers
if ($release.draft -or $release.prerelease) {
    throw "OBS companion release $($release.tag_name) is not a published stable release."
}

$requiredAssets = @(
    'vorti-obs-plugin-windows-x64.dll',
    'vorti-obs-plugin-windows-x64.dll.sha256',
    'release-compatibility.json'
)
$assets = @{}
foreach ($name in $requiredAssets) {
    $asset = @($release.assets | Where-Object name -eq $name)
    if ($asset.Count -ne 1) { throw "Release $($release.tag_name) must contain exactly one $name asset." }
    $assets[$name] = $asset[0]
}

$temporaryRoot = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { [IO.Path]::GetTempPath() }
$stagingDirectory = Join-Path $temporaryRoot "vortideck-obs-$($release.id)"
New-Item -ItemType Directory -Path $stagingDirectory -Force | Out-Null
foreach ($name in $requiredAssets) {
    Invoke-WebRequest -Uri $assets[$name].browser_download_url -Headers $headers -OutFile (Join-Path $stagingDirectory $name)
}

$dllPath = Join-Path $stagingDirectory 'vorti-obs-plugin-windows-x64.dll'
$expectedHashText = Get-Content -Raw -LiteralPath (Join-Path $stagingDirectory 'vorti-obs-plugin-windows-x64.dll.sha256')
$expectedHash = [regex]::Match($expectedHashText, '(?i)\b[a-f0-9]{64}\b').Value.ToUpperInvariant()
if (-not $expectedHash) { throw 'The companion checksum asset does not contain a SHA-256 value.' }
$actualHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $dllPath).Hash.ToUpperInvariant()
if ($actualHash -ne $expectedHash) { throw "OBS companion SHA-256 mismatch: expected $expectedHash, got $actualHash." }
if ($assets['vorti-obs-plugin-windows-x64.dll'].digest) {
    $apiHash = ($assets['vorti-obs-plugin-windows-x64.dll'].digest -replace '^sha256:', '').ToUpperInvariant()
    if ($apiHash -ne $actualHash) { throw "GitHub asset digest mismatch: expected $apiHash, got $actualHash." }
}

$manifestPath = Join-Path $stagingDirectory 'release-compatibility.json'
$manifest = Get-Content -Raw -LiteralPath $manifestPath | ConvertFrom-Json
$releaseVersion = $release.tag_name -replace '^v', ''
if ($manifest.schemaVersion -ne 1 -or $manifest.plugin.id -ne 'vorti-obs-plugin') {
    throw 'Unsupported OBS companion compatibility manifest.'
}
if ($manifest.plugin.version -ne $releaseVersion) {
    throw "Companion manifest version $($manifest.plugin.version) does not match release $($release.tag_name)."
}
$windowsArtifact = @($manifest.artifacts | Where-Object {
    $_.platform -eq 'windows' -and $_.architecture -eq 'x86_64' -and $_.kind -eq 'dll'
})
if ($windowsArtifact.Count -ne 1) { throw 'Manifest does not declare exactly one Windows x86_64 DLL.' }
if ($windowsArtifact[0].fileName -ne 'vorti-obs-plugin-windows-x64.dll') {
    throw 'Manifest Windows artifact filename does not match the required release asset.'
}
if ($windowsArtifact[0].sha256.ToUpperInvariant() -ne $actualHash) {
    throw 'Manifest Windows artifact checksum does not match the downloaded DLL.'
}
if ($manifest.source.commit -notmatch '^[a-fA-F0-9]{40}$') {
    throw 'Manifest does not contain a full source commit identifier.'
}

$vortideck = Get-Content -Raw -LiteralPath $VortiDeckConfig | ConvertFrom-Json
try {
    $currentVersion = [version]$vortideck.version
    $minimumVersion = [version]$manifest.vortideck.minimumVersion
} catch {
    throw "Release compatibility versions must be numeric semantic versions: $($_.Exception.Message)"
}
if ($currentVersion -lt $minimumVersion) {
    throw "OBS companion $($release.tag_name) requires VortiDeck $minimumVersion or newer; this build is $currentVersion."
}

$signature = Get-AuthenticodeSignature -LiteralPath $dllPath
if ($RequireAuthenticode -and $signature.Status -ne 'Valid') {
    throw "OBS companion Authenticode signature is not valid: $($signature.Status) $($signature.StatusMessage)"
}
if ($signature.Status -notin @('Valid', 'NotSigned')) {
    throw "OBS companion has an unacceptable Authenticode state: $($signature.Status) $($signature.StatusMessage)"
}

$destinationDirectory = Split-Path -Parent $Destination
if ($destinationDirectory) { New-Item -ItemType Directory -Path $destinationDirectory -Force | Out-Null }
Copy-Item -LiteralPath $dllPath -Destination $Destination -Force

if ($EmbeddedManifestDestination) {
    $embeddedManifest = [ordered]@{
        schemaVersion = 1
        id = 'vorti-obs-plugin'
        version = $manifest.plugin.version
        protocolVersion = $manifest.plugin.protocolVersion
        platform = 'windows'
        architecture = 'x86_64'
        fileName = 'vorti-obs-plugin.dll'
        sha256 = $actualHash
    }
    $embeddedManifestDirectory = Split-Path -Parent $EmbeddedManifestDestination
    if ($embeddedManifestDirectory) { New-Item -ItemType Directory -Path $embeddedManifestDirectory -Force | Out-Null }
    $embeddedManifest | ConvertTo-Json -Depth 10 | Set-Content -LiteralPath $EmbeddedManifestDestination -Encoding utf8
}

$provenance = [ordered]@{
    schemaVersion = 1
    repository = $Repository
    releaseTag = $release.tag_name
    releaseUrl = $release.html_url
    releaseId = $release.id
    sourceCommit = $manifest.source.commit
    artifact = 'vorti-obs-plugin-windows-x64.dll'
    architecture = 'x86_64'
    sha256 = $actualHash
    signatureStatus = $signature.Status.ToString()
    signer = if ($signature.SignerCertificate) { $signature.SignerCertificate.Subject } else { $null }
    compatibility = $manifest
}
$provenanceDirectory = Split-Path -Parent $ProvenancePath
if ($provenanceDirectory) { New-Item -ItemType Directory -Path $provenanceDirectory -Force | Out-Null }
$provenance | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $ProvenancePath -Encoding utf8
$signatureLabel = if ($signature.Status -eq 'Valid') { 'signed' } else { 'unsigned' }
Write-Host "Staged $signatureLabel OBS companion $($release.tag_name) ($actualHash)." -ForegroundColor Green
