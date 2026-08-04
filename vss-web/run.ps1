#!/usr/bin/env pwsh

[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(Position = 0, ValueFromRemainingArguments = $true)]
    [string[]]$Actions = @(),
    [string]$Remote = 'playground-web'
)

$ErrorActionPreference = 'Stop'

$ValidActions = @('deploy', 'watch', 'start')
$LocalUrl = 'http://localhost:5173'
$RemotePath = '/vss/'
$RemoteTarget = "${Remote}:$RemotePath"
$appDir = Join-Path $PSScriptRoot 'app'

function Show-Usage {
    Write-Host @"
Usage: .\run.ps1 <action> [<action> ...] [-Remote <name>]

Actions:
  watch         Run the Vite development server.
  deploy        Build and sync app\dist to $RemoteTarget.
  start         Open $LocalUrl, or the deployed URL after deploy.

Options:
  -Remote       rclone remote name (default: playground-web).
                The remote needs a web_url field for 'deploy start'. Set it with:
                rclone config update playground-web web_url "http://your-url-here/"

Examples:
  .\run.ps1 watch start
  .\run.ps1 deploy start
  .\run.ps1 deploy start -Remote another-remote
"@
}

function Invoke-Native {
    param([string]$Description, [scriptblock]$Command)
    & $Command
    if ($LASTEXITCODE -ne 0) {
        throw "$Description failed with exit code $LASTEXITCODE"
    }
}

function Assert-Tool {
    param([string]$Name)
    if (-not (Get-Command $Name -ErrorAction SilentlyContinue)) {
        throw "Missing required tool: $Name"
    }
}

function Get-RemoteWebUrl {
    $config = Invoke-Native 'Reading rclone configuration' { & rclone config show $Remote }
    $webUrl = $config | ForEach-Object {
        if ($_ -match '^\s*web_url\s*=\s*(.+?)\s*$') { $Matches[1] }
    } | Select-Object -First 1

    if (-not $webUrl) {
        throw "Remote '$Remote' has no web_url field"
    }

    return $webUrl.TrimEnd('/') + '/' + $RemotePath.Trim('/') + '/'
}

if ($Actions.Count -eq 0) {
    Show-Usage
    exit 0
}

foreach ($action in $Actions) {
    if ($ValidActions -notcontains $action) {
        Write-Host "Unknown action: $action"
        Show-Usage
        exit 1
    }
}

Push-Location $appDir
try {
    $deployed = $false
    $startHandled = $false

    foreach ($action in $Actions) {
        switch ($action) {
            'deploy' {
                Assert-Tool 'npm'
                Assert-Tool 'rclone'

                Write-Host 'Building web app...'
                Invoke-Native 'Web build' { & npm run build }

                Write-Host "Deploying dist to $RemoteTarget..."
                Invoke-Native 'Deployment' { & rclone sync 'dist' $RemoteTarget --progress }
                $deployed = $true
            }
            'watch' {
                Assert-Tool 'npm'
                Write-Host 'Starting development server...'
                if ($Actions -contains 'start') {
                    Start-Job -ScriptBlock {
                        param($Url)
                        Start-Sleep -Seconds 2
                        Start-Process $Url
                    } -ArgumentList $LocalUrl | Out-Null
                    $startHandled = $true
                }
                Invoke-Native 'Development server' { & npm run dev }
            }
            'start' {
                if ($startHandled) { continue }
                $url = if ($deployed) { Get-RemoteWebUrl } else { $LocalUrl }
                Write-Host "Opening $url..."
                Start-Process $url
                $startHandled = $true
            }
        }
    }
}
finally {
    Pop-Location
}
