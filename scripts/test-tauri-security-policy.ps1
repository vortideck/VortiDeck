$ErrorActionPreference = 'Stop'

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$tauriRoot = Join-Path $repositoryRoot 'apps\tauri-windows\src-tauri'

function Assert-SecurityPolicy {
    param([bool] $Condition, [string] $Message)
    if (-not $Condition) { throw "Tauri security policy failed: $Message" }
}

$config = Get-Content -Raw -LiteralPath (Join-Path $tauriRoot 'tauri.conf.json') | ConvertFrom-Json
$security = $config.app.security
$csp = $security.csp

Assert-SecurityPolicy ($null -ne $csp -and $csp -isnot [string]) 'Production CSP must be an enabled directive map, not null or the string "null".'
Assert-SecurityPolicy ($csp.'default-src' -match "'self'" -and $csp.'default-src' -match 'asset:') 'default-src must trust only packaged and asset-protocol content.'
Assert-SecurityPolicy ($csp.'connect-src' -match '(^|\s)ipc:' -and $csp.'connect-src' -match 'http://ipc\.localhost') 'connect-src must retain Tauri IPC endpoints.'
Assert-SecurityPolicy ($csp.'object-src' -eq "'none'") 'Object/embed content must be disabled.'
Assert-SecurityPolicy ($csp.'script-src' -notmatch "'unsafe-inline'|'unsafe-eval'") 'Application scripts must not permit unsafe-inline or unrestricted unsafe-eval.'
Assert-SecurityPolicy ($security.dangerousDisableAssetCspModification -ne $true) 'Tauri CSP nonce/hash injection must remain enabled.'

$defaultCapability = Get-Content -Raw -LiteralPath (Join-Path $tauriRoot 'capabilities\default.json') | ConvertFrom-Json
$simplePermissions = @($defaultCapability.permissions | Where-Object { $_ -is [string] })
Assert-SecurityPolicy ($simplePermissions -notcontains 'shell:allow-execute') 'Unscoped shell execution is forbidden; only the declared sidecar may execute.'
Assert-SecurityPolicy ($simplePermissions -notcontains 'shell:allow-spawn') 'Unscoped process spawning is forbidden; only the declared sidecar may spawn.'

$executeScopes = @($defaultCapability.permissions | Where-Object { $_.identifier -eq 'shell:allow-execute' })
$spawnScopes = @($defaultCapability.permissions | Where-Object { $_.identifier -eq 'shell:allow-spawn' })
Assert-SecurityPolicy ($executeScopes.Count -eq 1 -and $executeScopes[0].allow.Count -eq 1 -and $executeScopes[0].allow[0].sidecar) 'Execute scope must contain only the packaged node-runner sidecar.'
Assert-SecurityPolicy ($spawnScopes.Count -eq 1 -and $spawnScopes[0].allow.Count -eq 1 -and $spawnScopes[0].allow[0].sidecar) 'Spawn scope must contain only the packaged node-runner sidecar.'

$pluginCapability = Get-Content -Raw -LiteralPath (Join-Path $tauriRoot 'capabilities\plugin.json') | ConvertFrom-Json
$pluginPermissions = @($pluginCapability.permissions)
Assert-SecurityPolicy (-not ($pluginPermissions -match '^(fs|shell|process|http|dialog|opener|store|websocket):')) 'Plugin windows must not inherit filesystem, process, network, dialog, opener, or store privileges.'

$desktopManifest = Get-Content -Raw -LiteralPath (Join-Path $tauriRoot 'Cargo.toml')
Assert-SecurityPolicy ($desktopManifest -notmatch 'tauri\s*=\s*\{[^\r\n]*features\s*=\s*\[[^\]]*"devtools"') 'Production desktop builds must not compile the WebView devtools feature.'

Write-Host 'Tauri CSP and capability policy validation passed.' -ForegroundColor Green
