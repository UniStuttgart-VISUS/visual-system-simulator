[CmdletBinding(PositionalBinding = $false)]
param(
    [string]$Device = "",
    [Parameter(Position = 0, ValueFromRemainingArguments = $true)]
    [string[]]$Actions = @()
)

$ErrorActionPreference = "Stop"

$ValidActions = @("install", "start")
$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$iosDir = $scriptDir
$derivedDataDir = Join-Path $iosDir "build/DerivedData"
$app = Join-Path $derivedDataDir "Build/Products/Debug-iphoneos/VSS.app"
$deviceIdentifier = $null

function Show-Usage {
    Write-Host @"
Usage: ./run.ps1 <action> [<action> ...] [options]

Actions (run in the order given, any combination):
  install     Build and install the debug app on a physical iOS device.
  start       Stop an existing instance and launch the app.

Examples:
  ./run.ps1 install
  ./run.ps1 install start

Options:
  -Device <id|name>   Select a connected physical iOS device. Required when
                      more than one device is available.
"@
}

if ($Actions.Count -eq 0) { Show-Usage; exit 0 }
foreach ($action in $Actions) {
    if ($ValidActions -notcontains $action) {
        Write-Host "Unknown action: $action"
        Show-Usage
        exit 1
    }
}

function Invoke-Native {
    param([string]$Description, [scriptblock]$Command)
    $output = & $Command
    if ($LASTEXITCODE -ne 0) {
        $output | ForEach-Object { Write-Host $_ }
        throw "$Description failed with exit code $LASTEXITCODE"
    }
    return $output
}

function Get-PhysicalDevices {
    $destinations = Invoke-Native "Listing Xcode destinations" {
        & xcodebuild -project VSS.xcodeproj -scheme VSS -showdestinations
    }

    return @($destinations | ForEach-Object {
        $idMatch = [regex]::Match($_, "id:([^,}]+)")
        $nameMatch = [regex]::Match($_, "name:([^}]+)")
        if ($_ -match "platform:iOS," -and $_ -notmatch "placeholder|error:" -and
            $idMatch.Success -and $nameMatch.Success) {
            [PSCustomObject]@{
                Id = $idMatch.Groups[1].Value.Trim()
                Name = $nameMatch.Groups[1].Value.Trim()
            }
        }
    })
}

function Get-DeviceIdentifier {
    if ($script:deviceIdentifier) { return $script:deviceIdentifier }

    $devices = @(Get-PhysicalDevices)
    if ($Device) {
        $matches = @($devices | Where-Object { $_.Id -eq $Device -or $_.Name -eq $Device })
        if ($matches.Count -eq 0) { throw "Physical iOS device not found: $Device" }
        if ($matches.Count -gt 1) { throw "Multiple devices are named '$Device'; use the device ID." }
        $script:deviceIdentifier = $matches[0].Id
        return $script:deviceIdentifier
    }

    if ($devices.Count -eq 0) { throw "No physical iOS device is connected." }
    if ($devices.Count -gt 1) { throw "Multiple physical iOS devices are connected; use -Device <id|name>." }

    $script:deviceIdentifier = $devices[0].Id
    return $script:deviceIdentifier
}

function Invoke-InstallAction {
    $deviceId = Get-DeviceIdentifier
    Write-Host "Building VSS for device $deviceId..."
    Invoke-Native "Building the iOS app" {
        & xcodebuild -project VSS.xcodeproj -scheme VSS -configuration Debug `
            -destination "id=$deviceId" -derivedDataPath $derivedDataDir build
    } | Out-Null

    if (!(Test-Path $app -PathType Container)) { throw "App bundle not found: $app" }

    Write-Host "Installing $app..."
    Invoke-Native "Installing the iOS app" {
        & xcrun devicectl device install app --device $deviceId $app
    } | Out-Null
}

function Invoke-StartAction {
    $deviceId = Get-DeviceIdentifier
    if (!(Test-Path $app -PathType Container)) {
        throw "App bundle not found: $app. Run 'install' first."
    }
    $bundleIdentifier = & /usr/libexec/PlistBuddy -c "Print :CFBundleIdentifier" (Join-Path $app "Info.plist")
    if ($LASTEXITCODE -ne 0 -or !$bundleIdentifier) { throw "Cannot read the app bundle identifier from $app" }

    Write-Host "Starting $bundleIdentifier on device $deviceId..."
    Invoke-Native "Starting the iOS app" {
        & xcrun devicectl device process launch --device $deviceId --terminate-existing $bundleIdentifier
    } | Out-Null
}

Push-Location $iosDir
try {
    foreach ($action in $Actions) {
        switch ($action) {
            "install" { Invoke-InstallAction }
            "start" { Invoke-StartAction }
        }
    }
    Write-Host "Done."
}
finally {
    Pop-Location
}
