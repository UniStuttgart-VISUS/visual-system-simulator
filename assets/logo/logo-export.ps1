[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string] $BlendFile,

    [string] $StagingDirectory,

    [string] $OutputDirectory,

    [string] $OiiotoolCommand,

    [string] $OcioDisplay = "sRGB",

    [string] $OcioView = "AgX",

    [string] $OcioLook = "AgX - Medium High Contrast",

    [double] $OcioExposure = 0.72,

    [string] $BlenderCommand
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function New-Directory {
    param([string] $Path)

    [void](New-Item -ItemType Directory -Path $Path -Force)
}

function Invoke-Oiiotool {
    param([string[]] $Arguments)

    & $OiiotoolCommand @Arguments
    if ($LASTEXITCODE -ne 0) {
        throw "oiiotool failed with exit code $LASTEXITCODE."
    }
}

function Resize-Icon {
    param(
        [string] $Source,
        [string] $Destination,
        [int] $Size,
        [switch] $Opaque
    )

    New-Directory (Split-Path -Parent $Destination)
    $arguments = @($Source, "--resize:filter=lanczos3", "${Size}x${Size}")
    if ($Opaque) {
        $arguments += @("--ch", "R,G,B")
    }
    $arguments += @("-d", "uint8", "--dither", "-o", $Destination)
    Invoke-Oiiotool $arguments
}

function Write-Ico {
    param(
        [string[]] $PngPaths,
        [string] $Destination
    )

    New-Directory (Split-Path -Parent $Destination)
    $images = [System.Collections.Generic.List[byte[]]]::new()
    foreach ($path in $PngPaths) {
        $images.Add([System.IO.File]::ReadAllBytes($path))
    }
    $stream = [System.IO.File]::Create($Destination)
    $writer = [System.IO.BinaryWriter]::new($stream)
    try {
        $writer.Write([uint16]0)
        $writer.Write([uint16]1)
        $writer.Write([uint16]$images.Count)
        $offset = 6 + (16 * $images.Count)
        for ($index = 0; $index -lt $images.Count; $index++) {
            $size = [int][System.IO.Path]::GetFileNameWithoutExtension($PngPaths[$index]).Split("-")[-1]
            $dimension = if ($size -eq 256) { 0 } else { $size }
            $writer.Write([byte]$dimension)
            $writer.Write([byte]$dimension)
            $writer.Write([byte]0)
            $writer.Write([byte]0)
            $writer.Write([uint16]1)
            $writer.Write([uint16]32)
            $writer.Write([uint32]$images[$index].Length)
            $writer.Write([uint32]$offset)
            $offset += $images[$index].Length
        }
        foreach ($image in $images) {
            $writer.Write($image)
        }
    } finally {
        $writer.Dispose()
        $stream.Dispose()
    }
}

function Convert-Master {
    param(
        [string] $Role,
        [switch] $Opaque
    )

    $sourcePrefix = "$BaseName-master-$Role"
    $candidates = @(Get-ChildItem -LiteralPath $StagingDirectory -Filter "$sourcePrefix*.exr" |
        Where-Object {
            $_.BaseName -eq $sourcePrefix -or
            $_.BaseName.Substring($sourcePrefix.Length) -match '^\d+$'
        })
    if ($candidates.Count -eq 0) {
        throw "Missing compositor EXR master for role '$Role' in '$StagingDirectory'."
    }

    $selected = $candidates | Sort-Object LastWriteTime -Descending | Select-Object -First 1
    $stableSource = Join-Path $StagingDirectory "$sourcePrefix.exr"
    if ($selected.FullName -ne $stableSource) {
        Move-Item -LiteralPath $selected.FullName -Destination $stableSource -Force
    }
    $candidates |
        Where-Object { $_.FullName -ne $stableSource -and (Test-Path -LiteralPath $_.FullName) } |
        Remove-Item -Force

    $destination = Join-Path $StagingDirectory "$sourcePrefix.png"
    $exposureMultiplier = [Math]::Pow(2.0, $OcioExposure)
    $arguments = @(
        "--colorconfig", $OcioConfig,
        $stableSource,
        "--chnames", "R,G,B,A",
        "--mulc", "$exposureMultiplier,$exposureMultiplier,$exposureMultiplier,1",
        "--ociodisplay:from=Linear Rec.709:looks=$OcioLook`:unpremult=1",
        $OcioDisplay, $OcioView,
        "-d", "uint8", "--dither"
    )
    if ($Opaque) {
        $arguments += @("--ch", "R,G,B")
    }
    $arguments += @("-o", $destination)
    Invoke-Oiiotool $arguments
}

function Copy-BuildAsset {
    param(
        [string] $Source,
        [string] $Destination
    )

    New-Directory (Split-Path -Parent $Destination)
    Copy-Item -LiteralPath $Source -Destination $Destination -Force
}

$blendPath = [System.IO.Path]::GetFullPath($BlendFile)
if (-not (Test-Path -LiteralPath $blendPath -PathType Leaf)) {
    throw "Blend file does not exist: $blendPath"
}

$blendDirectory = Split-Path -Parent $blendPath
$BaseName = "logo"

if ([string]::IsNullOrWhiteSpace($StagingDirectory)) {
    $StagingDirectory = Join-Path $blendDirectory "icon-export\staging"
}
$StagingDirectory = [System.IO.Path]::GetFullPath($StagingDirectory)
if (-not (Test-Path -LiteralPath $StagingDirectory -PathType Container)) {
    throw "Blender staging directory does not exist: $StagingDirectory"
}

# Blender's File Output node writes the compositor items as linear EXR.
# oiiotool uses Blender's bundled OCIO config to apply the matching AgX
# view/display transform before creating and resizing the PNG assets.
if ([string]::IsNullOrWhiteSpace($BlenderCommand)) {
    $blenderOnPath = Get-Command "blender" -ErrorAction SilentlyContinue
    if ($blenderOnPath) {
        $BlenderCommand = $blenderOnPath.Source
    } else {
        $knownBlenderPaths = @(
            "C:\Program Files\Blender Foundation\Blender 5.2\blender.exe",
            "C:\Program Files\Blender Foundation\Blender 5.1\blender.exe",
            "C:\Program Files\Blender Foundation\Blender 5.0\blender.exe",
            "/Applications/Blender.app/Contents/MacOS/Blender"
        )
        $BlenderCommand = $knownBlenderPaths |
            Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
            Select-Object -First 1
    }
}
if ([string]::IsNullOrWhiteSpace($BlenderCommand) -or -not (Test-Path -LiteralPath $BlenderCommand -PathType Leaf)) {
    throw "Blender executable was not found. Pass -BlenderCommand with the Blender executable path."
}

if ([string]::IsNullOrWhiteSpace($OiiotoolCommand)) {
    $oiiotoolOnPath = Get-Command "oiiotool" -ErrorAction SilentlyContinue
    if ($oiiotoolOnPath) {
        $OiiotoolCommand = $oiiotoolOnPath.Source
    } else {
        $OiiotoolCommand = Get-ChildItem `
            -Path (Join-Path $env:APPDATA "Python\Python*\Scripts\oiiotool.exe") `
            -ErrorAction SilentlyContinue |
            Sort-Object FullName -Descending |
            Select-Object -First 1 -ExpandProperty FullName
    }
}
if ([string]::IsNullOrWhiteSpace($OiiotoolCommand) -or -not (Test-Path -LiteralPath $OiiotoolCommand -PathType Leaf)) {
    throw "oiiotool was not found. Install OpenImageIO or pass -OiiotoolCommand with its executable path."
}

$blenderRoot = Split-Path -Parent $BlenderCommand
$blenderVersionDirectory = Get-ChildItem -LiteralPath $blenderRoot -Directory |
    Where-Object { $_.Name -match '^\d+\.\d+$' } |
    Sort-Object Name -Descending |
    Select-Object -First 1
$OcioConfig = if ($blenderVersionDirectory) {
    Join-Path $blenderVersionDirectory.FullName "datafiles\colormanagement\config.ocio"
} else {
    $null
}
if (-not $OcioConfig -or -not (Test-Path -LiteralPath $OcioConfig -PathType Leaf)) {
    throw "Blender's OpenColorIO configuration was not found beside '$BlenderCommand'."
}

# Rendering through Blender's compositor creates the four linear EXR masters.
# Blender appends the frame number to File Output paths. oiiotool applies the
# same Blender OCIO display/view transform and writes stable PNG master names.
& $BlenderCommand --background $blendPath --render-frame 1
if ($LASTEXITCODE -ne 0) {
    throw "Blender icon master render failed with exit code $LASTEXITCODE."
}

Convert-Master "fullbleed" -Opaque
Convert-Master "freeform"
Convert-Master "foreground-color"
Convert-Master "foreground-mono"

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $blendDirectory "icon-export\dist"
}
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)
New-Directory $OutputDirectory

$masterFullbleed = Join-Path $StagingDirectory "$BaseName-master-fullbleed.png"
$masterFreeform = Join-Path $StagingDirectory "$BaseName-master-freeform.png"
$masterForegroundColor = Join-Path $StagingDirectory "$BaseName-master-foreground-color.png"
$masterForegroundMono = Join-Path $StagingDirectory "$BaseName-master-foreground-mono.png"

# Keep stable, exactly named master copies in the distribution tree.
$masterOutput = Join-Path $OutputDirectory "masters"
Copy-BuildAsset $masterFullbleed (Join-Path $masterOutput "$BaseName-master-fullbleed.png")
Copy-BuildAsset $masterFreeform (Join-Path $masterOutput "$BaseName-master-freeform.png")
Copy-BuildAsset $masterForegroundColor (Join-Path $masterOutput "$BaseName-master-foreground-color.png")
Copy-BuildAsset $masterForegroundMono (Join-Path $masterOutput "$BaseName-master-foreground-mono.png")

# Android ----------------------------------------------------------------------
$androidRoot = Join-Path $OutputDirectory "android"
$androidAssets = Join-Path $androidRoot "assets"
$androidRes = Join-Path $androidRoot "res"

$androidDensities = @(
    @{ Name = "mdpi"; Adaptive = 108; Legacy = 48 },
    @{ Name = "hdpi"; Adaptive = 162; Legacy = 72 },
    @{ Name = "xhdpi"; Adaptive = 216; Legacy = 96 },
    @{ Name = "xxhdpi"; Adaptive = 324; Legacy = 144 },
    @{ Name = "xxxhdpi"; Adaptive = 432; Legacy = 192 }
)

foreach ($density in $androidDensities) {
    $densityName = [string]$density.Name
    $adaptiveSize = [int]$density.Adaptive
    $legacySize = [int]$density.Legacy
    $assetForeground = Join-Path $androidAssets "$BaseName-android-foreground-color-$adaptiveSize.png"
    $assetBackground = Join-Path $androidAssets "$BaseName-android-background-$adaptiveSize.png"
    $assetMono = Join-Path $androidAssets "$BaseName-android-foreground-mono-$adaptiveSize.png"
    $assetLegacy = Join-Path $androidAssets "$BaseName-android-legacy-$legacySize.png"

    Resize-Icon $masterForegroundColor $assetForeground $adaptiveSize
    Resize-Icon $masterFullbleed $assetBackground $adaptiveSize -Opaque
    Resize-Icon $masterForegroundMono $assetMono $adaptiveSize
    Resize-Icon $masterFreeform $assetLegacy $legacySize

    $mipmap = Join-Path $androidRes "mipmap-$densityName"
    Copy-BuildAsset $assetForeground (Join-Path $mipmap "ic_launcher_foreground.png")
    Copy-BuildAsset $assetBackground (Join-Path $mipmap "ic_launcher_background.png")
    Copy-BuildAsset $assetMono (Join-Path $mipmap "ic_launcher_monochrome.png")
    Copy-BuildAsset $assetLegacy (Join-Path $mipmap "ic_launcher.png")
    Copy-BuildAsset $assetLegacy (Join-Path $mipmap "ic_launcher_round.png")
}

Resize-Icon $masterFullbleed (Join-Path $androidAssets "$BaseName-android-playstore-512.png") 512 -Opaque

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
    @{ Directory = "mipmap-anydpi-v26"; Xml = $adaptiveV26 },
    @{ Directory = "mipmap-anydpi-v33"; Xml = $adaptiveV33 }
)) {
    $directory = Join-Path $androidRes $variant.Directory
    New-Directory $directory
    Set-Content -LiteralPath (Join-Path $directory "ic_launcher.xml") -Value $variant.Xml -Encoding UTF8
    Set-Content -LiteralPath (Join-Path $directory "ic_launcher_round.xml") -Value $variant.Xml -Encoding UTF8
}

# Apple / Xcode ----------------------------------------------------------------
$appleRoot = Join-Path $OutputDirectory "apple"
$appIconSet = Join-Path $appleRoot "AppIcon.appiconset"
$iconComposerSources = Join-Path $appleRoot "IconComposerSources"

$iosDefaultName = "$BaseName-apple-default-1024.png"
Resize-Icon $masterFullbleed (Join-Path $appIconSet $iosDefaultName) 1024 -Opaque
Resize-Icon $masterFullbleed (Join-Path $iconComposerSources "$BaseName-apple-background-1024.png") 1024 -Opaque
Resize-Icon $masterForegroundColor (Join-Path $iconComposerSources "$BaseName-apple-foreground-color-1024.png") 1024
Resize-Icon $masterForegroundMono (Join-Path $iconComposerSources "$BaseName-apple-foreground-mono-1024.png") 1024

$appleContents = [ordered]@{
    images = @(
        [ordered]@{
            filename = $iosDefaultName
            idiom = "universal"
            platform = "ios"
            size = "1024x1024"
        }
    )
    info = [ordered]@{
        author = "xcode"
        version = 1
    }
}
$appleContents | ConvertTo-Json -Depth 8 |
    Set-Content -LiteralPath (Join-Path $appIconSet "Contents.json") -Encoding UTF8

# Legacy macOS iconset. Strict iconset names are required by iconutil.
$macIconSet = Join-Path $appleRoot "$BaseName-macos-app.iconset"
$macEntries = @(
    @{ Name = "icon_16x16.png"; Size = 16 },
    @{ Name = "icon_16x16@2x.png"; Size = 32 },
    @{ Name = "icon_32x32.png"; Size = 32 },
    @{ Name = "icon_32x32@2x.png"; Size = 64 },
    @{ Name = "icon_128x128.png"; Size = 128 },
    @{ Name = "icon_128x128@2x.png"; Size = 256 },
    @{ Name = "icon_256x256.png"; Size = 256 },
    @{ Name = "icon_256x256@2x.png"; Size = 512 },
    @{ Name = "icon_512x512.png"; Size = 512 },
    @{ Name = "icon_512x512@2x.png"; Size = 1024 }
)
foreach ($entry in $macEntries) {
    Resize-Icon $masterFreeform (Join-Path $macIconSet $entry.Name) ([int]$entry.Size)
}

$runningOnMac = [System.Runtime.InteropServices.RuntimeInformation]::IsOSPlatform(
    [System.Runtime.InteropServices.OSPlatform]::OSX
)
if ($runningOnMac -and (Get-Command "iconutil" -ErrorAction SilentlyContinue)) {
    $icnsPath = Join-Path $appleRoot "$BaseName-macos-app.icns"
    & iconutil --convert icns --output $icnsPath $macIconSet
    if ($LASTEXITCODE -ne 0) {
        throw "iconutil failed with exit code $LASTEXITCODE."
    }
}

# Windows ----------------------------------------------------------------------
$windowsRoot = Join-Path $OutputDirectory "windows"
$windowsAssets = Join-Path $windowsRoot "assets"
$windowsPngs = @()
foreach ($size in @(16, 20, 24, 30, 32, 40, 48, 60, 64, 72, 80, 96, 128, 256)) {
    $path = Join-Path $windowsAssets "$BaseName-windows-app-$size.png"
    Resize-Icon $masterFreeform $path $size
    $windowsPngs += $path
}
$windowsIco = Join-Path $windowsRoot "$BaseName-windows-app.ico"
Write-Ico $windowsPngs $windowsIco

# Linux / freedesktop hicolor --------------------------------------------------
$linuxRoot = Join-Path $OutputDirectory "linux"
$linuxAssets = Join-Path $linuxRoot "assets"
$hicolorRoot = Join-Path $linuxRoot "hicolor"
foreach ($size in @(16, 24, 32, 48, 64, 128, 256, 512)) {
    $asset = Join-Path $linuxAssets "$BaseName-linux-app-$size.png"
    Resize-Icon $masterFreeform $asset $size
    Copy-BuildAsset $asset (Join-Path $hicolorRoot "$($size)x$size\apps\$BaseName.png")
}

# Web --------------------------------------------------------------------------
$webRoot = Join-Path $OutputDirectory "web"
$faviconPngs = @()
foreach ($size in @(16, 32, 48)) {
    $path = Join-Path $webRoot "$BaseName-web-favicon-$size.png"
    Resize-Icon $masterFreeform $path $size
    $faviconPngs += $path
}
Write-Ico $faviconPngs (Join-Path $webRoot "$BaseName-web-favicon.ico")

Resize-Icon $masterFullbleed (Join-Path $webRoot "$BaseName-web-apple-touch-180.png") 180 -Opaque
Resize-Icon $masterFreeform (Join-Path $webRoot "$BaseName-web-any-192.png") 192
Resize-Icon $masterFreeform (Join-Path $webRoot "$BaseName-web-any-512.png") 512
Resize-Icon $masterFullbleed (Join-Path $webRoot "$BaseName-web-maskable-512.png") 512 -Opaque

$webManifest = [ordered]@{
    icons = @(
        [ordered]@{
            src = "$BaseName-web-any-192.png"
            sizes = "192x192"
            type = "image/png"
            purpose = "any"
        },
        [ordered]@{
            src = "$BaseName-web-any-512.png"
            sizes = "512x512"
            type = "image/png"
            purpose = "any"
        },
        [ordered]@{
            src = "$BaseName-web-maskable-512.png"
            sizes = "512x512"
            type = "image/png"
            purpose = "maskable"
        }
    )
}
$webManifest | ConvertTo-Json -Depth 8 |
    Set-Content -LiteralPath (Join-Path $webRoot "$BaseName-web-manifest-icons.json") -Encoding UTF8

$webHead = @"
<link rel="icon" href="$BaseName-web-favicon.ico" sizes="any">
<link rel="icon" type="image/png" href="$BaseName-web-favicon-32.png" sizes="32x32">
<link rel="apple-touch-icon" href="$BaseName-web-apple-touch-180.png" sizes="180x180">
"@
Set-Content -LiteralPath (Join-Path $webRoot "$BaseName-web-head.html") -Value $webHead -Encoding UTF8

# Machine-readable export inventory.
$inventory = [ordered]@{
    schema = 1
    basename = $BaseName
    blend_file = $blendPath
    generated_utc = [DateTime]::UtcNow.ToString("o")
    resampling = "OpenImageIO lanczos3"
    color_management = [ordered]@{
        config = $OcioConfig
        display = $OcioDisplay
        view = $OcioView
        look = $OcioLook
        exposure = $OcioExposure
    }
    output_directory = $OutputDirectory
    macos_icns_generated = (Test-Path -LiteralPath (Join-Path $appleRoot "$BaseName-macos-app.icns"))
    masters = [ordered]@{
        fullbleed = $masterFullbleed
        freeform = $masterFreeform
        foreground_color = $masterForegroundColor
        foreground_mono = $masterForegroundMono
    }
}
$inventory | ConvertTo-Json -Depth 8 |
    Set-Content -LiteralPath (Join-Path $OutputDirectory "$BaseName-icon-export.json") -Encoding UTF8

Write-Host "Icon export complete: $OutputDirectory"
if (-not $runningOnMac) {
    Write-Host "macOS .iconset generated. Run this script on macOS, or run iconutil there, to create the final .icns."
}



