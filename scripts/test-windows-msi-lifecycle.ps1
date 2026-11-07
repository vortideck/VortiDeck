param(
    [Parameter(Mandatory = $true)]
    [string] $MsiPath,
    [string] $PreviousMsiPath,
    [switch] $ExpectAuthenticode,
    [switch] $KeepInstalled
)

$ErrorActionPreference = 'Stop'
$principal = New-Object Security.Principal.WindowsPrincipal([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'MSI lifecycle acceptance must run from an elevated disposable Windows runner.'
}

$currentMsi = (Resolve-Path -LiteralPath $MsiPath).Path
$previousMsi = if ($PreviousMsiPath) { (Resolve-Path -LiteralPath $PreviousMsiPath).Path } else { $null }
$logDirectory = Join-Path $env:RUNNER_TEMP 'vortideck-msi-acceptance'
if (-not $env:RUNNER_TEMP) { $logDirectory = Join-Path $env:TEMP 'vortideck-msi-acceptance' }
New-Item -ItemType Directory -Path $logDirectory -Force | Out-Null

function Invoke-MsiExec {
    param([string[]] $Arguments, [string] $Name)
    $log = Join-Path $logDirectory "$Name.log"
    $process = Start-Process msiexec.exe -ArgumentList (@($Arguments) + @('/norestart', '/l*v', $log)) -Wait -PassThru
    if ($process.ExitCode -notin @(0, 3010, 1641)) {
        throw "MSI operation '$Name' failed with exit code $($process.ExitCode). Log: $log"
    }
}

function Assert-Authenticode {
    param([string] $Path)
    $signature = Get-AuthenticodeSignature -LiteralPath $Path
    if ($signature.Status -ne 'Valid') { throw "Expected a valid Authenticode signature on $Path, got $($signature.Status)." }
}

if ($ExpectAuthenticode) { Assert-Authenticode $currentMsi }

$obsDll = Join-Path $env:ProgramData 'obs-studio\plugins\vorti-obs-plugin\bin\64bit\vorti-obs-plugin.dll'
$installedExe = Join-Path $env:ProgramFiles 'VortiDeck\VortiDeck.exe'

if ($previousMsi) {
    Invoke-MsiExec -Arguments @('/i', $previousMsi, '/qn') -Name 'previous-install'
    Invoke-MsiExec -Arguments @('/i', $currentMsi, '/qn') -Name 'major-upgrade'
    if (Test-Path -LiteralPath $obsDll) { throw 'Major upgrade left the legacy MSI-owned OBS DLL behind.' }
} else {
    Invoke-MsiExec -Arguments @('/i', $currentMsi, '/qn') -Name 'clean-install'
}

if (-not (Test-Path -LiteralPath $installedExe)) { throw "VortiDeck executable was not installed at $installedExe" }
if (Test-Path -LiteralPath $obsDll) { throw 'App-only MSI unexpectedly installed an OBS companion DLL.' }
if ($ExpectAuthenticode) { Assert-Authenticode $installedExe }

Remove-Item -LiteralPath $installedExe -Force
Invoke-MsiExec -Arguments @('/fa', $currentMsi, '/qn') -Name 'repair'
if (-not (Test-Path -LiteralPath $installedExe)) { throw 'MSI repair did not restore the VortiDeck executable.' }

if (-not $KeepInstalled) {
    Invoke-MsiExec -Arguments @('/x', $currentMsi, '/qn') -Name 'uninstall'
    if (Test-Path -LiteralPath $installedExe) { throw 'MSI uninstall left the VortiDeck executable behind.' }
}

Write-Host "MSI lifecycle acceptance passed. Logs: $logDirectory" -ForegroundColor Green
