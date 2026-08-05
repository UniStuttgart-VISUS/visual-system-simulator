#!/usr/bin/env pwsh

[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(Position = 0, ValueFromRemainingArguments = $true)]
    [string[]]$Actions = @(),
    [string]$Remote = 'playground-web'
)

. "$PSScriptRoot/../scripts/run-common.ps1"

$LocalUrl = 'http://localhost:5173'
$RemotePath = '/vss/'
$RemoteTarget = "${Remote}:$RemotePath"
$appDir = Join-Path $PSScriptRoot 'app'

function Show-Usage {
    Write-Host @"
Usage: .\run.ps1 <action> [<action> ...] [-Remote <name>]

Actions:
  verify        Install missing dependencies, run tests, and build the production app.
  watch         Run the Vite development server.
  publish       Build and sync app\dist to $RemoteTarget.
  start         Open $LocalUrl, or the deployed URL after publish.

Options:
  -Remote       rclone remote name (default: playground-web).
                The remote needs a web_url field for 'publish start'. Set it with:
                rclone config update playground-web web_url "http://your-url-here/"

Examples:
  .\run.ps1 verify
  .\run.ps1 watch start
  .\run.ps1 publish start
  .\run.ps1 publish start -Remote another-remote
"@
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

$state = @{ Published = $false; StartHandled = $false }
$handlers = [ordered]@{
    verify = {
        Assert-Tool 'npm'
        if (!(Test-Path 'node_modules' -PathType Container)) {
            Write-Host 'Installing web dependencies...'
            Invoke-Native 'Web dependency installation' { & npm ci }
        }
        Invoke-Native 'Web tests' { & npm test }
        Invoke-Native 'Web production build' { & npm run build }
    }
    publish = {
        Assert-Tool 'npm'
        Assert-Tool 'rclone'
        Write-Host 'Building web app...'
        Invoke-Native 'Web build' { & npm run build }
        Write-Host "Publishing dist to $RemoteTarget..."
        Invoke-Native 'Web publication' { & rclone sync 'dist' $RemoteTarget --progress }
        $state.Published = $true
    }
    watch = {
        Assert-Tool 'npm'
        Write-Host 'Starting development server...'
        if ($Actions -contains 'start') {
            Start-Job -ScriptBlock {
                param($Url)
                Start-Sleep -Seconds 2
                Start-Process $Url
            } -ArgumentList $LocalUrl | Out-Null
            $state.StartHandled = $true
        }
        Invoke-Native 'Development server' { & npm run dev }
    }
    start = {
        if (-not $state.StartHandled) {
            $url = if ($state.Published) { Get-RemoteWebUrl } else { $LocalUrl }
            Write-Host "Opening $url..."
            Start-Process $url
            $state.StartHandled = $true
        }
    }
}

Invoke-VssActions -Actions $Actions -Handlers $handlers -Usage { Show-Usage } -WorkingDirectory $appDir
