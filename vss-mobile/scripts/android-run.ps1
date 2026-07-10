[CmdletBinding(PositionalBinding = $false)]
param(
    [string]$Device = "",
    [Parameter(Position = 0, ValueFromRemainingArguments = $true)]
    [string[]]$Actions = @(),
    [string]$Media = ""
)

$ErrorActionPreference = "Stop"

$ValidActions = @("install", "start", "camera", "share", "screenshot")
$TapX = 861; $TapY = 2043
$MediaStoreTimeoutSec = 30
$RenderTimeoutSec = 20
$StartSettleSeconds = 3

$scriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$mobileDir = Split-Path -Parent $scriptDir
$repoDir = Split-Path -Parent $mobileDir
$androidDir = Join-Path $mobileDir "android"
$launchDir = (Get-Location).Path
$apk = Join-Path $androidDir "build\outputs\apk\debug\VSS-debug.apk"
$adbArgs = @(); if ($Device) { $adbArgs = @("-s", $Device) }

$mediaTypes = @{
    ".jpg"  = @{ Kind = "image"; Mime = "image/jpeg" }
    ".jpeg" = @{ Kind = "image"; Mime = "image/jpeg" }
    ".png"  = @{ Kind = "image"; Mime = "image/png" }
    ".webp" = @{ Kind = "image"; Mime = "image/webp" }
    ".mp4"  = @{ Kind = "video"; Mime = "video/mp4" }
    ".mov"  = @{ Kind = "video"; Mime = "video/quicktime" }
    ".webm" = @{ Kind = "video"; Mime = "video/webm" }
}

function Show-Usage {
    Write-Host @"
Usage: .\android-run.ps1 <action> [<action> ...] [options]

Actions (run in the order given, any combination):
  install     Build and install the debug APK.
  start       Open com.vss/.MainActivity.
  camera      Start live camera simulation (tap the start button).
  share       Share a local image/video (see -Media) with com.vss/.MainActivity.
  screenshot  Capture a screenshot.
              After 'start': waits ~${StartSettleSeconds}s for the app to settle.
              After 'camera'/'share': waits for a rendered frame.

Examples:
  .\android-run.ps1 install start
  .\android-run.ps1 install start camera screenshot
  .\android-run.ps1 install start share screenshot -Media assets\marketplace.png

Options:
  -Device <serial>   Pass -s <serial> to adb.
  -Media <path>      Local image/video path, required for 'share'.
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
    if ($LASTEXITCODE -ne 0) { throw "$Description failed with exit code $LASTEXITCODE" }
    return $output
}

function Invoke-Adb {
    param([string]$Description, [string[]]$Arguments, [switch]$AllowFailure)
    if ($AllowFailure) { return & adb @adbArgs @Arguments 2>$null }
    return Invoke-Native $Description { & adb @adbArgs @Arguments }
}

function Wait-Until {
    param([string]$Description, [int]$TimeoutSec, [scriptblock]$Probe)
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        $result = & $Probe
        if ($result) { return $result }
        Start-Sleep -Milliseconds 250
    }
    throw "Timed out waiting for $Description"
}

function Wait-ForRenderedFrame {
    Write-Host "Waiting for one rendered frame..."
    Wait-Until "rendered frame log" $RenderTimeoutSec {
        (Invoke-Adb "Reading logcat" @("logcat", "-d")) |
        Select-String -Quiet -Pattern "External YCbCr render encoder|HardwareBuffer zero-copy path is active|RGBA image upload path is active"
    } | Out-Null
}

function Save-Screenshot {
    $timestamp = Get-Date -Format "yyyyMMdd-HHmmss"
    $remote = "/sdcard/vss-mobile-$timestamp.png"
    $local = Join-Path $scriptDir "vss-mobile-$timestamp.png"
    Write-Host "Capturing screenshot to $local..."
    Invoke-Adb "Capturing screenshot" @("shell", "screencap", "-p", $remote) | Out-Null
    Invoke-Adb "Pulling screenshot" @("pull", $remote, $local) | Out-Null
    Invoke-Adb "Removing remote screenshot" @("shell", "rm", $remote) | Out-Null
}

function Get-MediaInfo {
    param([string]$Path)
    $extension = [IO.Path]::GetExtension($Path).ToLowerInvariant()
    $info = $mediaTypes[$extension]
    if ($null -eq $info) { throw "Unsupported share media type: $extension" }
    return $info
}

function ConvertTo-RemoteFileName {
    param([string]$Path)
    $safeName = ([IO.Path]::GetFileName($Path)) -replace "[^A-Za-z0-9._-]", "_"
    return "vss-$(Get-Date -Format 'yyyyMMdd-HHmmss')-$safeName"
}

function ConvertTo-AdbShellArgument {
    param([string]$Value)
    if ($null -eq $Value) { return "''" }
    return "'" + ($Value -replace "'", "'\''") + "'"
}

function Invoke-AdbShellCommand {
    param([string[]]$Arguments, [switch]$AllowFailure)
    $command = ($Arguments | ForEach-Object { ConvertTo-AdbShellArgument $_ }) -join " "
    return Invoke-Adb -Description "Running adb shell command" -Arguments @("shell", $command) -AllowFailure:$AllowFailure
}

function Get-MediaStoreUriForRemoteMedia {
    param([string]$MediaKind, [string]$RemoteFileName)
    $baseUri = if ($MediaKind -eq "video") { "content://media/external/video/media" } else { "content://media/external/images/media" }
    $escapedFileName = $RemoteFileName -replace "'", "''"
    $where = "(relative_path='Pictures/VSS/' AND _display_name='$escapedFileName') OR (_data LIKE '%/Pictures/VSS/$escapedFileName')"

    $rows = Invoke-AdbShellCommand -AllowFailure -Arguments @("content", "query", "--uri", $baseUri, "--projection", "_id", "--where", $where, "--sort", "date_added DESC")
    if ($LASTEXITCODE -ne 0) { return $null }

    $match = $rows | Select-String -Pattern "_id=(\d+)" | Select-Object -First 1
    if ($match) { return "$baseUri/$($match.Matches[0].Groups[1].Value)" }
    return $null
}

function Wait-ForMediaStoreUri {
    param([string]$MediaKind, [string]$RemoteFileName)
    return Wait-Until "$RemoteFileName in MediaStore" $MediaStoreTimeoutSec {
        Get-MediaStoreUriForRemoteMedia $MediaKind $RemoteFileName
    }
}

function Push-LocalMediaForSharing {
    param([string]$Path)
    $localPath = @($Path, (Join-Path $launchDir $Path), (Join-Path $repoDir $Path)) |
    ForEach-Object { Resolve-Path $_ -ErrorAction SilentlyContinue } |
    Select-Object -First 1 -ExpandProperty Path -ErrorAction SilentlyContinue

    if (-not $localPath) { throw "Share path does not exist: $Path" }
    if (!(Test-Path $localPath -PathType Leaf)) { throw "Share path is not a file: $Path" }

    $mediaInfo = Get-MediaInfo $localPath
    $remoteFileName = ConvertTo-RemoteFileName $localPath
    $remoteDir = "/sdcard/Pictures/VSS"
    $remotePath = "$remoteDir/$remoteFileName"

    Write-Host "Pushing $localPath to $remotePath..."
    Invoke-Adb "Creating remote media directory" @("shell", "mkdir", "-p", $remoteDir) | Out-Null
    Invoke-Adb "Pushing media" @("push", $localPath, $remotePath) | Out-Null
    Invoke-Adb "Scanning media" @("shell", "am", "broadcast", "-a", "android.intent.action.MEDIA_SCANNER_SCAN_FILE", "-d", "file://$remotePath") | Out-Null

    return @{
        Uri      = Wait-ForMediaStoreUri $mediaInfo.Kind $remoteFileName
        MimeType = $mediaInfo.Mime
    }
}

function Invoke-InstallAction {
    Write-Host "Building debug APK..."
    Invoke-Native "Building debug APK" { & .\gradlew.bat --no-daemon assembleDebug } | Out-Null
    if (!(Test-Path $apk)) { throw "APK not found: $apk" }
    Write-Host "Installing $apk..."
    Invoke-Adb "Installing debug APK" @("install", "-r", $apk) | Out-Null
}

function Invoke-Start {
    Write-Host "Clearing logcat..."
    Invoke-Adb "Clearing logcat" @("logcat", "-c") | Out-Null
    Write-Host "Starting com.vss/.MainActivity..."
    Invoke-Adb "Stopping com.vss" @("shell", "am", "force-stop", "com.vss") | Out-Null
    Invoke-Adb "Starting com.vss/.MainActivity" @("shell", "am", "start", "-n", "com.vss/.MainActivity") | Out-Null
}

function Invoke-CameraAction {
    Write-Host "Tapping start button at $TapX,$TapY..."
    Start-Sleep -Seconds 2
    Invoke-Adb "Tapping start button" @("shell", "input", "tap", "$TapX", "$TapY") | Out-Null
}

function Invoke-ShareAction {
    if (-not $Media) { throw "share requires -Media <path>. Example: .\android-run.ps1 share -Media assets\marketplace.png" }

    Invoke-Adb "Clearing logcat" @("logcat", "-c") | Out-Null
    $sharedMedia = Push-LocalMediaForSharing $Media

    Write-Host "Sharing $($sharedMedia.Uri) to com.vss/.MainActivity..."
    Invoke-Adb "Sharing media to com.vss/.MainActivity" @(
        "shell", "am", "start",
        "-a", "android.intent.action.SEND",
        "-d", $sharedMedia.Uri,
        "-t", $sharedMedia.MimeType,
        "--eu", "android.intent.extra.STREAM", $sharedMedia.Uri,
        "--grant-read-uri-permission",
        "-n", "com.vss/.MainActivity"
    ) | Out-Null
}

function Invoke-ScreenshotAction {
    param([string]$LastAppAction)
    if ($LastAppAction -eq "start") {
        Write-Host "Waiting ${StartSettleSeconds}s for the app to settle..."
        Start-Sleep -Seconds $StartSettleSeconds
    }
    else {
        Wait-ForRenderedFrame
    }
    Save-Screenshot
}

Push-Location $androidDir
try {
    $lastAppAction = $null
    foreach ($action in $Actions) {
        switch ($action) {
            "install" { Invoke-InstallAction }
            "start" { Invoke-Start; $lastAppAction = "start" }
            "camera" { Invoke-CameraAction; $lastAppAction = "camera" }
            "share" { Invoke-ShareAction; $lastAppAction = "share" }
            "screenshot" { Invoke-ScreenshotAction -LastAppAction $lastAppAction }
        }
    }
    Write-Host "Done."
}
finally {
    Pop-Location
}
