# App Icon

`logo.blend` is the visual source for the application icons.
`logo-export.ps1` renders it and generates assets for Android, Apple, Windows,
Linux, and the web.

## Prerequisites

- Blender 5.2 LTS
- PowerShell
- [OpenImageIO](https://openimageio.readthedocs.io/) with `oiiotool`
- Write access to the staging and output directories

On macOS, the system-provided `iconutil` is also needed to create the final
`.icns` file.

Verify the tools:

```powershell
& "C:\Program Files\Blender Foundation\Blender 5.2\blender.exe" --version
oiiotool --version
```

## Export

Run from this folder:

```powershell
New-Item -ItemType Directory -Path .\export\staging -Force

.\logo-export.ps1 `
  -BlendFile .\logo.blend `
  -StagingDirectory .\export\staging `
  -OutputDirectory .\export\dist `
  -BlenderCommand "C:\Program Files\Blender Foundation\Blender 5.2\blender.exe"
```

Blender and `oiiotool` are auto-detected when possible. For all parameters and
defaults, run `Get-Help .\logo-export.ps1 -Full`.
Existing output files may be overwritten; obsolete files are not removed.

## Platform Requirements

### Android

- Adaptive icons use separate opaque background and transparent foreground
  layers at 108 × 108 dp.
- Keep important artwork inside the centered 66 × 66 dp safe zone.
- Do not pre-mask adaptive layers; the launcher applies its own mask.
- Android 13/API 33 themed icons require a monochrome layer.
- Google Play requires an opaque 512 × 512, 32-bit sRGB PNG no larger than
  1024 KB, without rounded corners or an outer drop shadow.

The current export uses the finished full-bleed icon as the adaptive
background. A proper background-only Blender master is still required to avoid
duplicating the foreground during launcher effects.

Sources:
[Adaptive icons](https://developer.android.com/develop/ui/compose/system/icon_design_adaptive),
[Google Play icon specification](https://developer.android.com/distribute/google-play/resources/icon-design-specifications)

### iOS, iPadOS, and macOS

- Use a square 1024 × 1024 source without baked-in rounded corners.
- Background layers must be full-bleed and opaque; foreground layers may use
  transparency.
- Provide suitable default and tinted/monochrome appearances when required.
- Use Xcode asset catalogs or Icon Composer; use `iconutil` on macOS for
  classic `.icns` files.

The export provides a default AppIcon, layer sources, and a classic macOS
`.iconset`. It does not create a finished Icon Composer file or dark/tinted
asset-catalog entries. Its Apple background currently has the same
background-only limitation as Android.

Sources:
[Apple App Icons](https://developer.apple.com/design/human-interface-guidelines/app-icons/),
[Xcode asset catalogs](https://developer.apple.com/documentation/xcode/configuring-your-app-icon/),
[Icon Composer](https://developer.apple.com/documentation/xcode/creating-your-app-icon-using-icon-composer)

### Windows

- Classic Win32 applications should provide a multi-size `.ico`; 16, 24, 32,
  48, and 256 px cover the common minimum.
- Transparent backgrounds are suitable for taskbar, Explorer, and shortcuts.
- Packaged WinUI/UWP/MSIX applications require manifest-bound PNG names and
  the appropriate theme variants.

The export supports classic `.ico` use. A complete MSIX/WinUI asset set,
including target-size names and light/dark/unplated variants, must be completed
separately.

Source:
[Windows app icon construction](https://learn.microsoft.com/en-us/windows/apps/design/iconography/app-icon-construction)

### Linux

- Install application icons in the fallback `hicolor` theme.
- Provide at least `hicolor/48x48/apps/<name>.png`; additional fixed sizes
  improve rendering.
- The icon name must match the `Icon=` entry in the `.desktop` file.

Sources:
[Icon Theme Specification](https://specifications.freedesktop.org/icon-theme/latest/index.html),
[Icon Naming Specification](https://specifications.freedesktop.org/icon-naming/latest/)

### Web and PWA

- Favicons should include at least 16 × 16 and 32 × 32.
- Installable Chromium-based PWAs need 192 × 192 and 512 × 512 icons.
- Maskable icons must be opaque and full-bleed, with important artwork inside
  the centered safe-zone circle whose radius is 40% of the image.
- Use a 180 × 180 `apple-touch-icon` for iOS web clips.
- Add generated icon entries to the application's complete web manifest; the
  export contains only the `icons` fragment.

Sources:
[Web App Manifest](https://www.w3.org/TR/appmanifest/),
[PWA manifest](https://web.dev/learn/pwa/web-app-manifest),
[Apple web clips](https://developer.apple.com/library/archive/documentation/AppleApplications/Reference/SafariWebContent/ConfiguringWebApplications/ConfiguringWebApplications.html)
