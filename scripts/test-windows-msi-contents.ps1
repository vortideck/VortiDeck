param(
    [Parameter(Mandatory = $true)]
    [string] $MsiPath
)

$ErrorActionPreference = 'Stop'
$msi = (Resolve-Path -LiteralPath $MsiPath).Path
$installer = New-Object -ComObject WindowsInstaller.Installer
$database = $installer.GetType().InvokeMember(
    'OpenDatabase',
    [System.Reflection.BindingFlags]::InvokeMethod,
    $null,
    $installer,
    @($msi, 0)
)

function Invoke-MsiQuery {
    param([string] $Sql, [int] $Columns = 1)
    try {
        $view = $database.GetType().InvokeMember('OpenView', 'InvokeMethod', $null, $database, @($Sql))
    } catch {
        throw "MSI query failed [$Sql]: $($_.Exception.Message)"
    }
    $view.GetType().InvokeMember('Execute', 'InvokeMethod', $null, $view, $null) | Out-Null
    $rows = @()
    while ($true) {
        $record = $view.GetType().InvokeMember('Fetch', 'InvokeMethod', $null, $view, $null)
        if (-not $record) { break }
        $row = for ($index = 1; $index -le $Columns; $index++) {
            $record.GetType().InvokeMember('StringData', 'GetProperty', $null, $record, @($index))
        }
        $rows += [pscustomobject]@{ Values = @($row) }
    }
    $view.GetType().InvokeMember('Close', 'InvokeMethod', $null, $view, $null) | Out-Null
    return $rows
}

function Assert-MsiContent {
    param([bool] $Condition, [string] $Message)
    if (-not $Condition) { throw "MSI content validation failed: $Message" }
}

$upgradeCode = Invoke-MsiQuery "SELECT ``Value`` FROM ``Property`` WHERE ``Property``='UpgradeCode'"
Assert-MsiContent ($upgradeCode.Count -eq 1 -and $upgradeCode[0].Values[0] -eq '{45733D75-581E-5E1E-BC63-5E9B937ADB6B}') 'UpgradeCode changed.'

$feature = Invoke-MsiQuery "SELECT ``Feature`` FROM ``Feature`` WHERE ``Feature``='ObsCompanionFeature'"
Assert-MsiContent ($feature.Count -eq 0) 'Obsolete MSI-owned OBS companion feature is still present.'

$files = Invoke-MsiQuery "SELECT ``File``, ``FileName``, ``Component_`` FROM ``File``" 3
$components = Invoke-MsiQuery "SELECT ``Component``, ``Directory_`` FROM ``Component``" 2
$directories = Invoke-MsiQuery "SELECT ``Directory``, ``Directory_Parent``, ``DefaultDir`` FROM ``Directory``" 3
$componentDirectory = @{}
foreach ($component in $components) { $componentDirectory[$component.Values[0]] = $component.Values[1] }
$directoryRows = @{}
foreach ($directory in $directories) { $directoryRows[$directory.Values[0]] = $directory.Values }

$obsFile = @($files | Where-Object { $_.Values[1] -match 'vorti-obs-plugin\.dll' })
Assert-MsiContent ($obsFile.Count -eq 0) 'MSI must not install a standalone OBS DLL; the payload is embedded in VortiDeck.'

$shellDll = @($files | Where-Object { $_.Values[1] -match 'vortideck-shell-ext\.dll' })
Assert-MsiContent ($shellDll.Count -eq 1) 'Shell extension DLL must have exactly one MSI owner.'

$shellIcon = @($files | Where-Object { $_.Values[1] -match '(^|\|)icon\.png$' })
Assert-MsiContent ($shellIcon.Count -eq 1) 'Sparse-package icon must have exactly one MSI owner.'
$shellDirectoryId = $componentDirectory[$shellDll[0].Values[2]]
$shellDirectory = $directoryRows[$shellDirectoryId]
$resourcesDirectory = $directoryRows[$shellDirectory[1]]
Assert-MsiContent ($shellDirectory[2] -match '(^|\|)shell-ext$' -and $resourcesDirectory[2] -match '(^|\|)resources$') 'Shell DLL is not installed at resources\shell-ext.'
Assert-MsiContent ($componentDirectory[$shellIcon[0].Values[2]] -eq $shellDirectoryId) 'Shell icon and DLL must share the runtime resource directory.'

$unregisterAction = Invoke-MsiQuery "SELECT ``Action`` FROM ``CustomAction`` WHERE ``Action``='UnregisterSparsePackage'"
Assert-MsiContent ($unregisterAction.Count -eq 1) 'Checked sparse-package uninstall cleanup is missing.'

$closeTable = Invoke-MsiQuery "SELECT ``Name`` FROM ``_Tables`` WHERE ``Name``='WixCloseApplication'"
Assert-MsiContent ($closeTable.Count -eq 0) 'MSI still owns OBS-running companion maintenance.'

$webViewBinary = Invoke-MsiQuery "SELECT ``Name`` FROM ``Binary`` WHERE ``Name``='MicrosoftEdgeWebview2Setup.exe'"
Assert-MsiContent ($webViewBinary.Count -eq 1) 'The WebView2 bootstrapper is not embedded in the MSI.'
$invokeBootstrapper = Invoke-MsiQuery "SELECT ``Action`` FROM ``CustomAction`` WHERE ``Action``='InvokeBootstrapper'"
Assert-MsiContent ($invokeBootstrapper.Count -eq 1) 'The embedded WebView2 bootstrapper custom action is missing.'
$downloadBootstrapper = Invoke-MsiQuery "SELECT ``Action`` FROM ``CustomAction`` WHERE ``Action``='DownloadAndInvokeBootstrapper'"
Assert-MsiContent ($downloadBootstrapper.Count -eq 0) 'MSI still contains the fragile download-and-invoke WebView2 custom action.'

Write-Host 'MSI database content validation passed.' -ForegroundColor Green
