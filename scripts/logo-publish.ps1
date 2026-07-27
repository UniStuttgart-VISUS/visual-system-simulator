[CmdletBinding()]
param(
    [string] $SourceDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

$RepositoryRoot = [System.IO.Path]::GetFullPath((Join-Path $PSScriptRoot ".."))

if ([string]::IsNullOrWhiteSpace($SourceDirectory)) {
    $SourceDirectory = Join-Path $RepositoryRoot "assets\logo\export\dist"
}
$SourceDirectory = [System.IO.Path]::GetFullPath($SourceDirectory)

if (-not (Test-Path -LiteralPath (Join-Path $RepositoryRoot "Cargo.toml") -PathType Leaf)) {
    throw "Repository root is not a VSS checkout: $RepositoryRoot"
}
if (-not (Test-Path -LiteralPath $SourceDirectory -PathType Container)) {
    throw "Logo export does not exist: $SourceDirectory"
}

function New-Directory {
    param([string] $Path)

    [void](New-Item -ItemType Directory -Path $Path -Force)
}

function Assert-RepositoryPath {
    param([string] $Path)

    $resolved = [System.IO.Path]::GetFullPath($Path)
    $prefix = $RepositoryRoot.TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar,
        [System.IO.Path]::AltDirectorySeparatorChar
    ) + [System.IO.Path]::DirectorySeparatorChar
    if (-not $resolved.StartsWith($prefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Publish destination is outside the repository: $resolved"
    }
}

function Reset-Directory {
    param([string] $Path)

    Assert-RepositoryPath $Path
    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
    New-Directory $Path
}

function Copy-RequiredFile {
    param(
        [string] $Source,
        [string] $Destination
    )

    if (-not (Test-Path -LiteralPath $Source -PathType Leaf)) {
        throw "Required logo asset is missing: $Source"
    }
    Assert-RepositoryPath $Destination
    New-Directory (Split-Path -Parent $Destination)
    Copy-Item -LiteralPath $Source -Destination $Destination -Force
}

# iOS: Xcode compiles and thins this asset catalog for the target device.
$iosSource = Join-Path $SourceDirectory "ios\AppIcon.appiconset"
$iosDestination = Join-Path $RepositoryRoot "vss-ios\App\Assets.xcassets\AppIcon.appiconset"
if (-not (Test-Path -LiteralPath $iosSource -PathType Container)) {
    throw "Required iOS app icon set is missing: $iosSource"
}
Reset-Directory $iosDestination
Copy-Item -Path (Join-Path $iosSource "*") -Destination $iosDestination -Recurse -Force

# Android: publish the rendered density assets and create the repository-specific
# adaptive-icon descriptors. Round icons reuse ic_launcher.
$androidSource = Join-Path $SourceDirectory "android\res"
$androidDestination = Join-Path $RepositoryRoot "vss-android\app\src\main\res"
$densityNames = @("mdpi", "hdpi", "xhdpi", "xxhdpi", "xxxhdpi")
$launcherFiles = @(
    "ic_launcher.png",
    "ic_launcher_background.png",
    "ic_launcher_foreground.png",
    "ic_launcher_monochrome.png"
)
foreach ($density in $densityNames) {
    $source = Join-Path $androidSource "mipmap-$density"
    $destination = Join-Path $androidDestination "mipmap-$density"
    Reset-Directory $destination
    foreach ($name in $launcherFiles) {
        Copy-RequiredFile (Join-Path $source $name) (Join-Path $destination $name)
    }
}
$adaptiveV26 = @'
<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@mipmap/ic_launcher_background" />
    <foreground android:drawable="@mipmap/ic_launcher_foreground" />
</adaptive-icon>
'@

$adaptiveV33 = @'
<?xml version="1.0" encoding="utf-8"?>
<adaptive-icon xmlns:android="http://schemas.android.com/apk/res/android">
    <background android:drawable="@mipmap/ic_launcher_background" />
    <foreground android:drawable="@mipmap/ic_launcher_foreground" />
    <monochrome android:drawable="@mipmap/ic_launcher_monochrome" />
</adaptive-icon>
'@

foreach ($variant in @(
    @{ Api = "v26"; Xml = $adaptiveV26 },
    @{ Api = "v33"; Xml = $adaptiveV33 }
)) {
    $destinationDirectory = Join-Path $androidDestination "mipmap-anydpi-$($variant.Api)"
    Reset-Directory $destinationDirectory
    Set-Content `
        -LiteralPath (Join-Path $destinationDirectory "ic_launcher.xml") `
        -Value $variant.Xml `
        -Encoding UTF8
}

# Web: keep tracked inputs outside public because prepare.mjs recreates public.
$webSource = Join-Path $SourceDirectory "web"
$webAssets = Join-Path $RepositoryRoot "vss-web\app\assets"
$webIcons = Join-Path $webAssets "icons"
Reset-Directory $webIcons
$webIconNames = @(
    "icon-192.png",
    "icon-512.png",
    "apple-touch-icon-180.png",
    "favicon-32.png",
    "favicon.ico",
    "icon-maskable-512.png"
)
foreach ($name in $webIconNames) {
    Copy-RequiredFile (Join-Path $webSource $name) (Join-Path $webIcons $name)
}
$webManifest = [ordered]@{
    name = "Visual System Simulator"
    short_name = "VSS"
    start_url = "."
    display = "standalone"
    background_color = "#071019"
    theme_color = "#071019"
    icons = @(
        [ordered]@{
            src = "icons/icon-192.png"
            sizes = "192x192"
            type = "image/png"
            purpose = "any"
        },
        [ordered]@{
            src = "icons/icon-512.png"
            sizes = "512x512"
            type = "image/png"
            purpose = "any"
        },
        [ordered]@{
            src = "icons/icon-maskable-512.png"
            sizes = "512x512"
            type = "image/png"
            purpose = "maskable"
        }
    )
}
$manifestPath = Join-Path $webAssets "manifest.webmanifest"
Assert-RepositoryPath $manifestPath
$webManifest | ConvertTo-Json -Depth 8 |
    Set-Content -LiteralPath $manifestPath -Encoding UTF8

# Desktop packaging consumes only the current operating system's subtree.
$desktopIcons = Join-Path $RepositoryRoot "vss-desktop\packaging\icons"
Reset-Directory $desktopIcons
Copy-RequiredFile `
    (Join-Path $SourceDirectory "windows\app.ico") `
    (Join-Path $desktopIcons "windows\app.ico")

$linuxSource = Join-Path $SourceDirectory "linux\hicolor"
$linuxDestination = Join-Path $desktopIcons "linux\hicolor"
if (-not (Test-Path -LiteralPath $linuxSource -PathType Container)) {
    throw "Required Linux hicolor icon tree is missing: $linuxSource"
}
New-Directory $linuxDestination
Copy-Item -Path (Join-Path $linuxSource "*") -Destination $linuxDestination -Recurse -Force

$macSource = Join-Path $SourceDirectory "macos\app.iconset"
$macDestination = Join-Path $desktopIcons "macos\app.iconset"
if (-not (Test-Path -LiteralPath $macSource -PathType Container)) {
    throw "Required macOS iconset is missing: $macSource"
}
New-Directory $macDestination
Copy-Item -Path (Join-Path $macSource "*") -Destination $macDestination -Recurse -Force

Write-Host "Published logo assets from '$SourceDirectory'."
