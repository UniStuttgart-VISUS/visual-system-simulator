[CmdletBinding()]
param(
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
        [switch] $Opaque,
        [switch] $OpaqueRgba
    )

    if ($Opaque -and $OpaqueRgba) {
        throw "Opaque and OpaqueRgba are mutually exclusive."
    }

    New-Directory (Split-Path -Parent $Destination)
    $arguments = @($Source, "--resize:filter=lanczos3", "${Size}x${Size}")
    if ($Opaque) {
        $arguments += @("--ch", "R,G,B")
    } elseif ($OpaqueRgba) {
        # Keep an explicit, fully opaque alpha channel. Google Play requires a
        # 32-bit PNG even though every pixel must be opaque.
        $arguments += @("--ch", "R,G,B,A=1.0")
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

if ([string]::IsNullOrWhiteSpace($BlendFile)) {
    $BlendFile = Join-Path $PSScriptRoot "logo.blend"
}
$blendPath = [System.IO.Path]::GetFullPath($BlendFile)
if (-not (Test-Path -LiteralPath $blendPath -PathType Leaf)) {
    throw "Blend file does not exist: $blendPath"
}

$blendDirectory = Split-Path -Parent $blendPath
$BaseName = "logo"

if ([string]::IsNullOrWhiteSpace($StagingDirectory)) {
    $StagingDirectory = Join-Path $blendDirectory "export\staging"
}
$StagingDirectory = [System.IO.Path]::GetFullPath($StagingDirectory)

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

New-Directory $StagingDirectory

# Rendering through Blender's compositor creates the six linear EXR masters.
# Blender appends the frame number to File Output paths. oiiotool applies the
# same Blender OCIO display/view transform and writes stable PNG master names.
& $BlenderCommand `
    --background $blendPath `
    --render-output (Join-Path $StagingDirectory "$BaseName-preview-") `
    --render-frame 1
if ($LASTEXITCODE -ne 0) {
    throw "Blender icon master render failed with exit code $LASTEXITCODE."
}

Convert-Master "fullbleed" -Opaque
Convert-Master "squircle"
Convert-Master "background" -Opaque
Convert-Master "foreground-color"
Convert-Master "foreground-monochrome"
Convert-Master "maskable" -Opaque

if ([string]::IsNullOrWhiteSpace($OutputDirectory)) {
    $OutputDirectory = Join-Path $blendDirectory "export\dist"
}
$OutputDirectory = [System.IO.Path]::GetFullPath($OutputDirectory)
$stagingPrefix = $StagingDirectory.TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
$outputPrefix = $OutputDirectory.TrimEnd("\", "/") + [System.IO.Path]::DirectorySeparatorChar
if ($OutputDirectory -eq $StagingDirectory -or
    $OutputDirectory.StartsWith($stagingPrefix, [System.StringComparison]::OrdinalIgnoreCase) -or
    $StagingDirectory.StartsWith($outputPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
    throw "Staging and output directories must not overlap."
}
New-Directory $OutputDirectory

$masterFullbleed = Join-Path $StagingDirectory "$BaseName-master-fullbleed.png"
$masterSquircle = Join-Path $StagingDirectory "$BaseName-master-squircle.png"
$masterBackground = Join-Path $StagingDirectory "$BaseName-master-background.png"
$masterForegroundColor = Join-Path $StagingDirectory "$BaseName-master-foreground-color.png"
$masterForegroundMonochrome = Join-Path $StagingDirectory "$BaseName-master-foreground-monochrome.png"
$masterMaskable = Join-Path $StagingDirectory "$BaseName-master-maskable.png"

# Android ----------------------------------------------------------------------
$androidRoot = Join-Path $OutputDirectory "android"
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
    $mipmap = Join-Path $androidRes "mipmap-$densityName"
    Resize-Icon $masterForegroundColor (Join-Path $mipmap "ic_launcher_foreground.png") $adaptiveSize
    Resize-Icon $masterBackground (Join-Path $mipmap "ic_launcher_background.png") $adaptiveSize -Opaque
    Resize-Icon $masterForegroundMonochrome (Join-Path $mipmap "ic_launcher_monochrome.png") $adaptiveSize
    Resize-Icon $masterSquircle (Join-Path $mipmap "ic_launcher.png") $legacySize
}

Resize-Icon `
    $masterFullbleed `
    (Join-Path $androidRoot "play-store-icon-512.png") `
    512 `
    -OpaqueRgba

# Apple / Xcode ----------------------------------------------------------------
$iosRoot = Join-Path $OutputDirectory "ios"
$appIconSet = Join-Path $iosRoot "AppIcon.appiconset"

$iosDefaultName = "app-icon-default-1024.png"
Resize-Icon $masterFullbleed (Join-Path $appIconSet $iosDefaultName) 1024 -Opaque

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
$macosRoot = Join-Path $OutputDirectory "macos"
$macIconSet = Join-Path $macosRoot "app.iconset"
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
New-Directory $macIconSet
New-Directory (Join-Path $StagingDirectory "macos")
foreach ($size in ($macEntries.Size | Sort-Object -Unique)) {
    Resize-Icon `
        $masterSquircle `
        (Join-Path $StagingDirectory "macos\icon-$size.png") `
        $size
}
foreach ($entry in $macEntries) {
    Copy-Item `
        -LiteralPath (Join-Path $StagingDirectory "macos\icon-$($entry.Size).png") `
        -Destination (Join-Path $macIconSet $entry.Name)
}

# Windows ----------------------------------------------------------------------
$windowsRoot = Join-Path $OutputDirectory "windows"
$windowsStaging = Join-Path $StagingDirectory "windows"
$windowsPngs = @()
foreach ($size in @(16, 24, 32, 48, 256)) {
    $path = Join-Path $windowsStaging "$BaseName-windows-app-$size.png"
    Resize-Icon $masterSquircle $path $size
    $windowsPngs += $path
}
$windowsIco = Join-Path $windowsRoot "app.ico"
Write-Ico $windowsPngs $windowsIco

# Linux / freedesktop hicolor --------------------------------------------------
$linuxRoot = Join-Path $OutputDirectory "linux"
$hicolorRoot = Join-Path $linuxRoot "hicolor"
foreach ($size in @(16, 24, 32, 48, 64, 128, 256, 512)) {
    Resize-Icon $masterSquircle `
        (Join-Path $hicolorRoot "$($size)x$size\apps\vss.png") `
        $size
}

# Web --------------------------------------------------------------------------
$webRoot = Join-Path $OutputDirectory "web"
$webStaging = Join-Path $StagingDirectory "web"
$faviconPngs = @()
foreach ($size in @(16, 32, 48)) {
    $path = Join-Path $webStaging "favicon-$size.png"
    Resize-Icon $masterSquircle $path $size
    $faviconPngs += $path
}
Write-Ico $faviconPngs (Join-Path $webRoot "favicon.ico")
Copy-Item `
    -LiteralPath (Join-Path $webStaging "favicon-32.png") `
    -Destination (Join-Path $webRoot "favicon-32.png")

Resize-Icon $masterFullbleed (Join-Path $webRoot "apple-touch-icon-180.png") 180 -Opaque
Resize-Icon $masterSquircle (Join-Path $webRoot "icon-192.png") 192
Resize-Icon $masterSquircle (Join-Path $webRoot "icon-512.png") 512
Resize-Icon $masterMaskable (Join-Path $webRoot "icon-maskable-512.png") 512 -Opaque

Write-Host "Icon export complete: $OutputDirectory"
