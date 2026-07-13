# Changelog

## Unreleased

VSS makes a hard configuration break. Engine inputs are **parameters** owned by
`vss`; JSON/UI entries are **settings**, UI ordering uses **groups**, and named partial
layers are **presets**, all owned by `vss-catalog`. Parsing, layering, diagnostics,
provenance, sidecar policy, and relative asset resolution moved out of the renderer.
Node configuration now uses typed, node-local parameter IDs and a central registry; the Inspector traversal has been removed.

Old: `{"both":{"simulator":{"Retina":{"glaucoma_onoff":true}},"pose":{"gaze":[960,540]}}}`

New: `{"both":{"glaucoma.enabled":true,"gaze":[960,540]},"left":{},"right":{}}`

There is no 1.x parser, adapter, or parameter alias. Rust consumers use typed
`ParameterId<T>`/`ParameterPatch`, or compile setting layers through `vss-catalog`.
Desktop owns file I/O and diagnostic presentation. Android exchanges setting IDs and
JSON values; profiles were renamed to presets.

| Old key | Setting ID | Parameter ID |
|---|---|---|
| `ct_onoff` | `cataract.enabled` | `cataract.enabled` |
| `ct_blur_factor` | `cataract.blur` | `cataract.blur` |
| `ct_contrast_factor` | `cataract.contrast` | `cataract.contrast` |
| `peacock_cb_onoff` | `color.enabled` | `color.enabled` |
| `peacock_cb_strength` | `color.strength` | `color.strength` |
| `peacock_cb_type` | `color.type` | `color.type` |
| `glaucoma_onoff` | `glaucoma.enabled` | `glaucoma.enabled` |
| `glaucoma_fov` | `glaucoma.field` | `glaucoma.field` |
| `achromatopsia_onoff` | `achromatopsia.enabled` | `achromatopsia.enabled` |
| `achromatopsia_int` | `achromatopsia.intensity` | `achromatopsia.intensity` |
| `achromatopsia_blur_factor` | `achromatopsia.blur` | `achromatopsia.blur` |
| `nyctalopia_onoff` | `nyctalopia.enabled` | `nyctalopia.enabled` |
| `nyctalopia_int` | `nyctalopia.intensity` | `nyctalopia.intensity` |
| `maculardegeneration_onoff` | `macular.enabled` | `macular.enabled` |
| `maculardegeneration_veasy` | `macular.simple` | `macular.simple` |
| `maculardegeneration_inteasy` | `macular.simple-intensity` | `macular.simple-intensity` |
| `maculardegeneration_vadvanced` | `macular.advanced` | `macular.advanced` |
| `maculardegeneration_radius` | `macular.radius` | `macular.radius` |
| `maculardegeneration_intadvanced` | `macular.intensity` | `macular.intensity` |
| pose `gaze` | `gaze` | `gaze` |
| pose `view` | `view` | `view` |

Research, diagnostic, and deferred overlay fields use dotted kebab-case IDs derived
from their node and old field names; they are not compatibility aliases.
