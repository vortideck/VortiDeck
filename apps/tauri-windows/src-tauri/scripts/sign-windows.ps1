param(
    [Parameter(Mandatory = $true, Position = 0)]
    [string[]] $Path,
    [switch] $RequireSignature
)

$ErrorActionPreference = 'Stop'

function Resolve-SignTool {
    $command = Get-Command signtool.exe -ErrorAction SilentlyContinue
    if ($command) { return $command.Source }

    $kitsRoot = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits\10\bin'
    if (Test-Path -LiteralPath $kitsRoot) {
        $candidate = Get-ChildItem -LiteralPath $kitsRoot -Directory |
            Sort-Object Name -Descending |
            ForEach-Object { Join-Path $_.FullName 'x64\signtool.exe' } |
            Where-Object { Test-Path -LiteralPath $_ } |
            Select-Object -First 1
        if ($candidate) { return $candidate }
    }

    throw 'signtool.exe was not found. Install the Windows SDK signing tools.'
}

$thumbprint = $env:WINDOWS_CODESIGN_CERTIFICATE_SHA1
$pfxPath = $env:WINDOWS_CODESIGN_PFX_PATH
$pfxPassword = $env:WINDOWS_CODESIGN_PFX_PASSWORD
$timestampUrl = if ($env:WINDOWS_CODESIGN_TIMESTAMP_URL) {
    $env:WINDOWS_CODESIGN_TIMESTAMP_URL
} else {
    'http://timestamp.digicert.com'
}

if (-not $thumbprint -and -not $pfxPath) {
    if ($RequireSignature -or $env:VORTIDECK_REQUIRE_WINDOWS_SIGNING -eq '1') {
        throw 'Windows signing is required, but neither WINDOWS_CODESIGN_CERTIFICATE_SHA1 nor WINDOWS_CODESIGN_PFX_PATH is configured.'
    }
    Write-Warning 'No Authenticode identity is configured; leaving local development artifacts unsigned.'
    exit 0
}

$signTool = Resolve-SignTool
foreach ($item in $Path) {
    $resolved = (Resolve-Path -LiteralPath $item).Path
    $arguments = @('sign', '/fd', 'SHA256', '/td', 'SHA256', '/tr', $timestampUrl)
    if ($thumbprint) {
        $arguments += @('/sha1', $thumbprint)
    } else {
        $arguments += @('/f', (Resolve-Path -LiteralPath $pfxPath).Path)
        if ($pfxPassword) { $arguments += @('/p', $pfxPassword) }
    }
    $arguments += $resolved

    & $signTool @arguments
    if ($LASTEXITCODE -ne 0) { throw "Authenticode signing failed for $resolved" }

    & $signTool verify /pa /all $resolved
    if ($LASTEXITCODE -ne 0) { throw "Authenticode verification failed for $resolved" }
}
