param(
    [Parameter(Mandatory = $true)]
    [string]$ExecutablePath
)

$ErrorActionPreference = 'Stop'
$resolvedExecutable = (Resolve-Path -LiteralPath $ExecutablePath).Path
$testRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("vortideck-packaged-recovery-" + [guid]::NewGuid().ToString('N'))
$dataRoot = Join-Path $testRoot 'data'
New-Item -ItemType Directory -Path $dataRoot -Force | Out-Null

function Stop-AcceptanceProcess {
    param([System.Diagnostics.Process]$Process)
    if ($null -ne $Process -and -not $Process.HasExited) {
        Stop-Process -Id $Process.Id -Force
        $Process.WaitForExit()
    }
}

function Invoke-AcceptancePhase {
    param(
        [Parameter(Mandatory = $true)][string]$Scenario,
        [Parameter(Mandatory = $true)][string]$MarkerName
    )

    $markerPath = Join-Path $testRoot $MarkerName
    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $resolvedExecutable
    $startInfo.UseShellExecute = $false
    $startInfo.Environment['VORTIDECK_ACCEPTANCE_DATA_DIR'] = $dataRoot
    $startInfo.Environment['VORTIDECK_ACCEPTANCE_SCENARIO'] = $Scenario
    $startInfo.Environment['VORTIDECK_ACCEPTANCE_MARKER'] = $markerPath
    $process = [System.Diagnostics.Process]::Start($startInfo)

    try {
        $deadline = [DateTime]::UtcNow.AddSeconds(45)
        while (-not (Test-Path -LiteralPath $markerPath)) {
            if ($process.HasExited) {
                throw "Acceptance host exited before '$Scenario' completed (exit code $($process.ExitCode))."
            }
            if ([DateTime]::UtcNow -ge $deadline) {
                throw "Timed out waiting for packaged acceptance phase '$Scenario'."
            }
            Start-Sleep -Milliseconds 100
        }

        $payload = Get-Content -Raw -LiteralPath $markerPath | ConvertFrom-Json
        if (-not $payload.success) {
            throw "Packaged acceptance phase '$Scenario' failed: $($payload.error)"
        }
        return @{ Process = $process; Payload = $payload }
    } catch {
        Stop-AcceptanceProcess -Process $process
        throw
    }
}

try {
    $seed = Invoke-AcceptancePhase -Scenario 'seed-crash' -MarkerName 'seed.json'
    Stop-AcceptanceProcess -Process $seed.Process

    $firstRecovery = Invoke-AcceptancePhase -Scenario 'recover' -MarkerName 'recovery-1.json'
    if ($firstRecovery.Payload.recoveredThisStartup -ne 1 -or
        $firstRecovery.Payload.recoveryRuns -ne 1 -or
        $firstRecovery.Payload.openApprovalWakes -ne 1) {
        throw "First restart did not restore exactly one run and approval wake: $($firstRecovery.Payload | ConvertTo-Json -Compress)"
    }
    Stop-AcceptanceProcess -Process $firstRecovery.Process

    $secondRecovery = Invoke-AcceptancePhase -Scenario 'recover' -MarkerName 'recovery-2.json'
    if ($secondRecovery.Payload.recoveredThisStartup -ne 0 -or
        $secondRecovery.Payload.recoveryRuns -ne 1 -or
        $secondRecovery.Payload.openApprovalWakes -ne 1) {
        throw "Second restart lost or duplicated durable recovery state: $($secondRecovery.Payload | ConvertTo-Json -Compress)"
    }
    if ($secondRecovery.Payload.runIds[0] -ne 'acceptance-run-before-process-kill' -or
        $secondRecovery.Payload.wakeIds[0] -ne 'acceptance-approval-wake') {
        throw "Recovered identifiers do not match the committed records."
    }
    Stop-AcceptanceProcess -Process $secondRecovery.Process

    Write-Host 'Packaged workflow crash/restart acceptance passed.' -ForegroundColor Green
} finally {
    Get-Process -Name 'VortiDeck' -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -eq $resolvedExecutable } |
        Stop-Process -Force -ErrorAction SilentlyContinue
    if (Test-Path -LiteralPath $testRoot) {
        Remove-Item -LiteralPath $testRoot -Recurse -Force
    }
}
