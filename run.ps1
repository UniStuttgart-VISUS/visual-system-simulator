#!/usr/bin/env pwsh

param(
    [Parameter(Position = 0)]
    [string]$Target = '',
    [Parameter(Position = 1, ValueFromRemainingArguments = $true)]
    [object[]]$TargetArguments = @()
)

$ErrorActionPreference = 'Stop'

$targets = [ordered]@{
    android = 'vss-android/run.ps1'
    desktop = 'vss-desktop/run.ps1'
    ios = 'vss-ios/run.ps1'
    web = 'vss-web/run.ps1'
}

function Show-Usage {
    Write-Host @"
Usage: ./run.ps1 <target> <action> [<action> ...] [options]

Targets:
  android     Android app and physical-device tools.
  desktop     Native desktop app.
  ios         iOS app and physical-device tools.
  web         Web app and playground deployment.

Common action:
  verify      Run the meaningful device-free checks for the target.

Examples:
  ./run.ps1 android install start
  ./run.ps1 desktop start
  ./run.ps1 ios install start
  ./run.ps1 web watch start

Run ./run.ps1 <target> without an action for target-specific help.
"@
}

if (-not $Target) {
    Show-Usage
    exit 0
}
if (-not $targets.Contains($Target)) {
    Show-Usage
    throw "Unknown target: $Target"
}

$targetScript = Join-Path $PSScriptRoot $targets[$Target]
$powerShell = (Get-Process -Id $PID).Path
& $powerShell -NoLogo -NoProfile -File $targetScript @TargetArguments
exit $LASTEXITCODE
