#!/usr/bin/env pwsh

[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(Position = 0)]
    [string]$Action = '',
    [Parameter(Position = 1, ValueFromRemainingArguments = $true)]
    [string[]]$AppArguments = @()
)

. "$PSScriptRoot/../scripts/run-common.ps1"

function Show-Usage {
    Write-Host @"
Usage: ./run.ps1 <action> [application arguments]

Actions:
  verify      Run the desktop tests on the current platform.
  start       Build and start the release profile.

Examples:
  ./run.ps1 verify
  ./run.ps1 start
  ./run.ps1 start show ../assets/cube.color.png
"@
}

$handlers = [ordered]@{
    verify = {
        Assert-Tool 'cargo'
        Invoke-Native 'Desktop verification' { & cargo test -p vss-desktop }
    }
    start = {
        Assert-Tool 'cargo'
        Invoke-Native 'Starting the desktop app' {
            & cargo run -p vss-desktop --release -- @AppArguments
        }
    }
}

$actions = if ($Action) { @($Action) } else { @() }
Invoke-VssActions -Actions $actions -Handlers $handlers -Usage { Show-Usage } -WorkingDirectory $PSScriptRoot
