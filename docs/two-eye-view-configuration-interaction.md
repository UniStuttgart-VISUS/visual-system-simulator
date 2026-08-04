# Two-Eye View, Configuration, and Interaction

## Problem Statement

The simulator models two eyes, but its non-OpenXR clients do not expose that model consistently. Desktop currently offers Left, Right, and Both configuration targets, while its normal window renders only one flow. Web, Android, and iOS expose neither an eye selector nor a two-eye view. As a result, users can configure state they cannot see, eye-specific demonstrations such as strabismus lose their intended meaning, and there is no non-XR side-by-side mode suitable for comparing both simulated eyes or placing the output in a simple Cardboard-style viewer.

The inconsistency extends beyond layout. Presets are compiled into one flat value map even though their source format contains `both`, `left`, and `right` sections. The current compiler merges the shared and left sections and discards the right section. Client session models therefore cannot reliably represent a symmetric preset applied to the currently viewed eye, a deliberately asymmetric preset, and independent manual overrides at the same time.

Input behavior is also fragmented. A two-eye view must retain one intentional gaze and one intentional camera direction across both eyes, work with ordinary mouse and touch input, and support 360-degree media without introducing head tracking, IPD, lens distortion, or accidental page scrolling. Fullscreen behavior and media changes must remain predictable on every platform.

## Solution

Introduce one non-XR eye mode with exactly three choices in the fixed order Left, Both, Right. The selector is both the visible view and the edit target: Left shows and edits the effective left eye, Right shows and edits the effective right eye, and Both renders the two effective eyes side by side while editing the shared layer. No non-XR state can be edited through a target that is not represented by the current view.

Keep two long-lived eye flows and one synchronized media producer. In Both mode, compose the left eye into the left half and the right eye into the right half of the same output. Both eyes start from the same source frame, base pose, view direction, projection, and field of view. Differences arise only from the configured simulation. Single-eye modes render only their selected flow. OpenXR remains runtime-driven and does not use this selector or the split-screen composition.

Replace the compiled flat preset contract with its intentional structured form. Every catalog preset must be either symmetric (`both`) or intrinsically eye-specific (`left`, `right`, or both `left` and `right`), never a mixture of shared and eye-specific sections. Symmetric presets apply to the current edit target. Intrinsically eye-specific presets retain their authored distribution and automatically select Both so their result is immediately visible.

Add compact platform-appropriate controls and shared semantic interactions. Mouse users drag with the left button to change gaze and with the right button to rotate the camera. Touch users drag with one finger to change gaze and with two fingers to rotate the camera. Deltas are relative to the whole preview. A mouse double-click or one-finger double-tap resets both camera and gaze to the configured starting pose.

## User Stories

1. As a non-XR user, I want to choose Left, Both, or Right, so that I can inspect the eye result relevant to me.
2. As a user in Left mode, I want the settings to edit the left-eye result I can see, so that the interface never edits an unrelated hidden target.
3. As a user in Right mode, I want the settings to edit the right-eye result I can see, so that the interface remains spatially and conceptually consistent.
4. As a user in Both mode, I want the settings to edit the shared layer while both eyes are visible, so that symmetric changes can be compared immediately.
5. As a first-time user without eye-specific configuration, I want the simulator to start in Left mode, so that the default view uses the full preview area.
6. As a user loading a configuration with left- or right-specific content, I want the simulator to start in Both mode, so that asymmetric state is visible immediately.
7. As a user, I want my chosen eye mode to remain stable while I change settings and media, so that the interface does not move unexpectedly.
8. As a user starting a new app session, I want eye mode to return to its content-sensitive default, so that stale display state is not persisted across restarts.
9. As a user, I want the eye selector ordered Left, Both, Right, so that its spatial order matches the output.
10. As a user, I want Left on the physical left and Right on the physical right in split screen, so that the composition is unambiguous.
11. As a user, I want the split to be an exact horizontal half-and-half layout, so that both eyes receive equal space.
12. As a user, I want no divider or gutter between the halves, so that no output area is wasted in a simple viewer.
13. As a user, I want unused output areas cleared to black, so that previous frames or eye modes never remain visible.
14. As a user, I want each half to retain the platform's existing Fit or Fill policy, so that two-eye support does not silently change media presentation.
15. As a user rotating a device or opening rotated media, I want fitting to use the oriented logical dimensions, so that portrait and landscape content are not stretched or cropped using stale dimensions.
16. As a user, I want both eye flows to receive the same decoded frame and timestamp, so that motion does not diverge between eyes.
17. As a user switching eye modes, I want the switch to be immediate, so that flows do not need to be rebuilt or media reopened.
18. As a user, I accept the memory cost of two long-lived flows, so that switching remains stable and deterministic.
19. As a user, I want split screen to retain full internal render resolution, so that adding the split does not introduce a new quality policy.
20. As a simple Cardboard user, I want both eyes to begin from the same camera and projection state, so that the images do not conflict unless the simulation deliberately differs.
21. As a user, I want the projection aspect ratio in Both mode to be derived from one half viewport, so that the view is not horizontally compressed.
22. As a user, I want IPD and stereo parallax omitted, so that the feature remains a simulation comparison rather than pretending to be calibrated stereo VR.
23. As a user, I want lens distortion omitted, so that the feature does not claim headset-specific optical correction.
24. As an OpenXR user, I want the runtime to continue determining its view count and viewports, so that native XR behavior remains intact.
25. As an OpenXR user, I do not want the non-XR eye selector or split-screen composition, so that hardware-driven views are not confused with the fallback display mode.
26. As a user activating strabismus or another asymmetric demonstration, I want Both mode selected automatically, so that the demonstration's two-eye relationship is visible.
27. As a user disabling the last asymmetric preset, I want the current eye mode to remain unchanged, so that the interface does not jump without an explicit selection.
28. As a user with an active asymmetric preset, I want to switch to one eye temporarily, so that I can inspect that eye in detail.
29. As a user activating a symmetric preset in Left mode, I want it applied to the left layer only, so that preset actions follow what I am viewing and editing.
30. As a user activating a symmetric preset in Right mode, I want it applied to the right layer only, so that I can configure the eyes independently.
31. As a user activating a symmetric preset in Both mode, I want it applied to the shared layer, so that it affects both eyes together.
32. As a user, I want article preset controls to omit redundant Left, Both, and Right labels, so that the current eye selector remains the single clear target indicator.
33. As a user, I want an article's active preset state to reflect the current target, so that selection markers describe the values I am editing.
34. As a user, I want a preset to be structurally either shared or eye-specific, so that contradictory ownership cannot enter the catalog.
35. As a catalog maintainer, I want empty presets rejected, so that every preset produces a meaningful simulation.
36. As a catalog maintainer, I want a preset mixing `both` with `left` or `right` rejected during the build, so that invalid state cannot reach clients.
37. As a catalog maintainer, I want a preset containing `left` and `right` accepted, so that asymmetric demonstrations can author both halves together.
38. As a strabismus user, I want both authored eye sections preserved, so that opposing eye-axis and center-distance values are not discarded.
39. As a user, I want independent per-eye selections and manual overrides retained within the session, so that one eye can have a different cataract strength from the other.
40. As a user editing a setting in Both mode, I want eye-specific overrides of that same setting removed, so that the newly entered shared value truly applies to both eyes.
41. As a user editing one shared setting, I want unrelated eye-specific settings to remain, so that a cataract edit does not erase strabismus geometry.
42. As a user activating a symmetric preset in Both mode, I want colliding eye-specific contributions removed only for settings owned by that preset, so that the preset's visible promise is fulfilled without broad data loss.
43. As a user, I want existing conflict confirmation for replaced manual values to remain available where appropriate, so that consequential replacement is understandable.
44. As a user resetting a setting in Left mode, I want only its left-layer value removed, so that the eye inherits the shared value again.
45. As a user resetting a setting in Right mode, I want only its right-layer value removed, so that the eye inherits the shared value again.
46. As a user resetting a setting in Both mode, I want only its shared-layer value removed, so that deliberate left and right overrides remain intact.
47. As a mouse user, I want left-button dragging to change gaze, so that I can steer visual intent continuously.
48. As a mouse user, I want right-button dragging to rotate the camera, so that gaze and scene navigation have distinct gestures.
49. As a touch user, I want one-finger dragging to change gaze, so that the primary simulation interaction is easy to reach.
50. As a touch user, I want two-finger centroid dragging to rotate the camera, so that scene navigation does not conflict with gaze.
51. As a user, I want drag gestures to apply relative deltas rather than absolute tap positions, so that the current gaze remains the stable starting point.
52. As a user, I want input to accumulate after release, so that the pose does not snap back when a gesture ends.
53. As a mouse user, I want a double-click to reset camera and gaze, so that I can recover the configured starting pose quickly.
54. As a touch user, I want a one-finger double-tap to reset camera and gaze, so that reset is available without extra screen controls.
55. As a user, I want both eye flows to receive the same intentional camera and gaze deltas, so that any later divergence comes from the simulation rather than input timing.
56. As a split-screen user, I want the entire preview to be the interaction surface, so that sensitivity does not double when a gesture begins over one half.
57. As a user dragging across the full preview width, I want a 180-degree yaw change, so that horizontal sensitivity is predictable.
58. As a user dragging across the full preview height, I want a 90-degree pitch change, so that vertical sensitivity is controlled.
59. As a user dragging right, I want camera or gaze direction to move right, so that the gesture follows direct manipulation expectations.
60. As a user dragging up, I want camera or gaze direction to move up, so that vertical interaction is natural.
61. As a user rotating horizontally, I want yaw to wrap cyclically, so that 360-degree media can be explored continuously.
62. As a user rotating vertically, I want camera and gaze pitch clamped near the poles, so that the view cannot flip over.
63. As a mobile or web user starting a gesture inside the preview, I want the preview to own that gesture, so that the article list does not scroll accidentally.
64. As a page-scrolling user, I want gestures beginning outside the preview to retain ordinary scrolling, so that the surrounding content remains usable.
65. As a user, I want preview overlay buttons to consume their own taps, so that pressing a control never changes gaze or camera.
66. As a user, I do not want keyboard gaze or camera shortcuts, so that keyboard behavior remains small and discoverable.
67. As a user opening 360-degree equirectangular media, I want all platforms to recognize and project it, so that it is not fitted as a flat 2:1 image.
68. As a 360-degree split-screen user, I want both eyes to sample the same sphere and base direction, so that the comparison remains coherent.
69. As a mobile 360-degree user, I want drag navigation without head tracking, so that the feature remains explicit and controllable.
70. As a web, Android, or iOS user, I want a compact three-icon eye selector over the preview, so that the control does not interrupt the camera, article, and settings layout.
71. As a touch user, I want the eye selector immediately left of the fullscreen button, so that preview controls form one predictable cluster.
72. As a user, I want original Cardboard-inspired icons with left, both, or right lenses highlighted, so that the three modes are recognizable without permanent labels.
73. As a user with color-vision limitations, I want selection shown by background or border as well as lens color, so that state does not depend on color alone.
74. As a screen-reader user, I want explicit localized names for all three eye modes, so that icon-only controls remain accessible.
75. As a pointer user, I want tooltips for the eye icons, so that their meaning can be learned before selection.
76. As a fullscreen mobile or web user, I want source, eye, and fullscreen overlay controls hidden, so that the output is unobstructed.
77. As a fullscreen user who wants another eye mode, I want to leave fullscreen before changing it, so that there is only one compact control layout.
78. As a desktop user, I want eye mode, Open File, and Fullscreen in one block above settings, so that output controls remain visible without overlaying the preview.
79. As a desktop user, I want Tab to hide or show the complete right-side control menu, so that I can maximize the preview in windowed and fullscreen modes.
80. As a desktop fullscreen user, I want Escape to exit fullscreen, so that it follows the standard platform convention.
81. As a desktop windowed user, I want Escape to do nothing, so that it no longer unexpectedly hides the menu.
82. As a desktop user, I want menu visibility preserved across fullscreen transitions, so that fullscreen does not overwrite my layout preference.
83. As a desktop user, I want hiding the menu to expand and recompute the preview viewports, so that the reclaimed space is actually used.
84. As a desktop user, I want to open a new media file at runtime, so that I do not need to restart the simulator.
85. As a web fullscreen user, I want the browser's standard Escape behavior to exit fullscreen, so that custom key handling is unnecessary.
86. As an Android fullscreen user, I want the Back action to exit fullscreen, so that preview taps remain available for simulation input.
87. As an iOS fullscreen user, I want the native back or edge gesture to exit fullscreen, so that preview taps remain available for simulation input.
88. As a fullscreen user, I do not want clicking or tapping the preview to exit fullscreen, so that gaze interaction is not stolen.
89. As a Both-mode fullscreen user on mobile or web, I want a best-effort landscape request, so that each eye receives a useful horizontal area.
90. As a single-eye fullscreen user, I want the current device orientation respected, so that no area is sacrificed merely to force landscape.
91. As a user whose platform rejects the landscape request, I want fullscreen to continue without an error, so that orientation policy does not block simulation.
92. As a user opening new media, I want eye mode and simulation settings preserved, so that I can compare sources under the same configuration.
93. As a user opening new media, I want fullscreen and desktop menu state preserved, so that the surrounding presentation remains stable.
94. As a user opening new media, I want camera and gaze offsets reset, so that the new source starts from its configured base pose.
95. As a user opening new media, I want RGBD and equirectangular metadata redetected, so that the new source uses the correct projection.
96. As a user cancelling a file picker, I want no state change, so that cancellation is harmless.
97. As a user whose new media fails to load, I want the prior medium retained with a visible error, so that a recoverable failure does not blank the simulator.
98. As a user changing media in Both mode, I want the new source switched atomically for both eye flows, so that halves never display different media.

## Implementation Decisions

- The product vocabulary distinguishes an **eye** (Left or Right), an **eye mode** (Left, Both, or Right), a configuration **layer** (`both`, `left`, or `right`), and an effective eye configuration. Both is an eye mode and layer target, not a third eye.
- The non-XR eye-mode enum has exactly three values and a stable visual order: Left, Both, Right. It defaults to Left unless initial configuration contains any left- or right-layer content, in which case it defaults to Both. It is session-only.
- Eye mode couples output and editing. Left and Right show one effective eye and edit that eye's layer. Both shows both effective eyes and edits the shared layer. Article controls and setting provenance derive from the current layer rather than displaying a second target selector.
- Each non-XR surface creates two eye flows once. Single-eye modes render only the selected flow; Both renders both. OpenXR continues to create and render the runtime-provided view count and bypasses the non-XR selector.
- Media ingestion is a producer shared by both flows. A decoded image or video frame is immutable or reference-counted so both active flows consume the same frame and timestamp without duplicating a decoder or stealing from a single-consumer queue.
- Both mode uses two normalized horizontal viewports: left `[0, 0, 0.5, 1]` and right `[0.5, 0, 0.5, 1]`. The frame is cleared to black once before composing; subsequent display passes load existing output. There is no adaptive vertical layout, divider, gutter, or headset mask.
- Existing platform render-resolution policy remains. Split screen changes only final display composition and does not halve internal eye-flow resolution or add dynamic resolution behavior.
- Existing platform Fit or Fill behavior remains, but it operates within the selected viewport and uses oriented logical source dimensions. Source contracts carry raw width, raw height, and rotation. Any effective size or rotation change triggers slot renegotiation.
- Both eyes use dual-mono geometry: identical source frame, timestamp, base camera pose, intentional gaze, view direction, field of view, and projection definition. The projection aspect is calculated from the half viewport. Only simulation configuration may produce an inter-eye difference.
- Equirectangular recognition becomes source metadata shared by all clients rather than a desktop-only filename side effect. Raw 2:1 panoramic media is projected onto the sphere. Both eye flows sample the same sphere and intentional base direction.
- The catalog's compiled preset representation preserves structured `both`, `left`, and `right` maps. The former flat `values` representation and any redundant `requiresBoth` marker are removed in one coordinated client contract change.
- Preset validation enforces exactly one authoring family: `both XOR (left OR right)`. Valid shapes are `both` only, `left` only, `right` only, or `left` plus `right`. Empty presets and any mixture of `both` with `left` or `right` fail catalog compilation with an actionable diagnostic.
- A `both` preset describes symmetric values but is applied to the current edit layer. An eye-specific preset has fixed authored eye ownership, updates those eye layers, and selects Both on activation. Deactivation never changes eye mode automatically.
- Session simulation state retains preset selections, masked fallback values, and manual overrides independently for shared, left, and right layers. Effective Left is shared composed with left; effective Right is shared composed with right.
- Applying a value manually to Both removes left and right contributions for that setting only. Applying a symmetric preset to Both likewise replaces colliding eye-specific contributions only for settings owned by that preset. Unrelated per-eye values survive.
- Reset is layer-local. Resetting on Both removes the shared contribution and exposes any per-eye overrides. Resetting on Left or Right removes that eye contribution and restores inheritance from Both.
- Preset activation state in article demonstrations is evaluated against the current target. The article UI does not display eye ownership labels. Invalid multi-target preset placement is a state-model or catalog error, not another UI state.
- Camera and gaze offsets are one shared intentional pose input. Semantic input events are `gaze_delta`, `view_delta`, and `reset_pose`; platform UI layers convert native gestures into these events.
- Input deltas are normalized by the entire preview's current width and height, including Both mode. A full-width horizontal drag is 180 degrees and a full-height vertical drag is 90 degrees. Right and up drags increase rightward and upward intent. Yaw wraps; camera and gaze pitch clamp to ±89 degrees.
- Mouse mapping is left-button drag for gaze, right-button drag for view, and double-click for reset. Touch mapping is one-finger pan for gaze, two-finger centroid pan for view, and one-finger double-tap for reset. Input is relative and accumulated for the session.
- Gesture recognition uses platform-native facilities. Web uses Pointer Events, pointer identity, and pointer capture with `touch-action: none` on the preview. Android uses Compose/native pointer and gesture APIs. iOS uses one- and two-touch pan recognizers plus a double-tap recognizer. The rendering core does not implement a shared raw-touch state machine, and Hammer.js is not introduced.
- Gestures beginning inside the preview are consumed until completion. Gestures beginning outside retain normal page scrolling. Overlay controls intercept their own interactions before the preview gesture layer.
- Camera and gaze offsets reset on explicit double-click/double-tap and successful media replacement. There are no keyboard pose controls and no head tracking.
- Web, Android, and iOS place an icon-only three-part segmented control inside the preview at bottom right, immediately left of fullscreen. Source selection remains bottom left. The icons are original shared artwork inspired by a neutral viewer silhouette, with left, both, or right lens fill. Selection also changes container background or border. Localized accessibility labels are mandatory; web/desktop pointer surfaces provide tooltips.
- Mobile and web hide source, eye-mode, and fullscreen controls in fullscreen. Changing eye mode requires leaving fullscreen. Preview clicks/taps never exit fullscreen.
- Desktop places Left/Both/Right, Open File, and Fullscreen in one non-scrolling control block above settings. Tab toggles the entire right menu in both windowed and fullscreen modes. Escape exits fullscreen when active and otherwise does nothing. Menu visibility persists across fullscreen transitions.
- Android Back exits fullscreen. iOS uses the platform-native back/edge route. Web relies on the browser's fullscreen Escape mechanism rather than custom Escape handling.
- Entering fullscreen while Both is selected requests landscape on a best-effort basis and releases that request on exit. Android uses a temporary activity orientation request, web attempts a fullscreen orientation lock and unlock, and iOS requests scene geometry where supported. Rejection is ignored. Single-eye fullscreen does not request landscape.
- Media replacement is transactional. It preserves eye mode, layer state, fullscreen, and desktop menu visibility; resets pose offsets; redetects source projection metadata; and publishes the new source to both flows atomically. Cancellation is a no-op. Failure retains the previous source and reports an error.
- The initial implementation must include original scalable eye-mode artwork and platform translations. IPD, headset lens profiles, and distortion calibration are not prerequisites for the icon or split-screen design.

## Testing Decisions

- Tests assert externally observable state transitions and serialized contracts. They avoid private widget structure, exact implementation class names, and incidental render-node wiring.
- The highest catalog seam is preset compilation and serialization. Contract tests cover all four valid structured shapes, reject empty or mixed-family shapes, prove that both halves of an asymmetric preset survive compilation, and verify that every client deserializes the new contract.
- Existing catalog consistency and build-time validation tests are the prior art for preset-shape validation, identifiers, setting references, localized articles, and demonstration references.
- The highest state seam is a pure session transition API shared in behavior across clients. Its table-driven tests cover initial eye-mode choice, target-relative symmetric presets, automatic Both selection for intrinsic eye-specific presets, manual overrides, masked values, setting-scoped collision removal, layer-local resets, per-target active article state, and effective left/right composition.
- State tests explicitly cover: activate strabismus, auto-select Both, switch to Left, apply cataract, and verify that cataract targets Left while the authored right-eye strabismus values remain.
- State tests explicitly cover independent left and right variants, a Both manual edit clearing only matching eye overrides, a Both preset replacing only owned settings, and resetting Both without deleting per-eye overrides.
- The highest rendering seam is the surface/display composition boundary. Tests or a deterministic render harness verify normalized full, left-half, and right-half viewports; one black clear; non-destructive second-eye composition; preserved Fit/Fill behavior; half-viewport projection aspect; and no stale pixels after mode or menu-size changes.
- Shared-media tests prove that two active eye flows observe the same frame identity/timestamp, one-eye rendering does not require a second decode, source replacement is atomic, cancellation is a no-op, and failure preserves the previous medium.
- Orientation tests cover 0, 90, 180, and 270 degrees, including a rotation-only metadata change with unchanged raw dimensions. They verify logical size renegotiation and correct Fit/Fill in portrait and landscape fullscreen.
- Equirectangular tests cover metadata transport on desktop, web, Android, and iOS; projection rather than flat fitting; synchronized sphere sampling; and shared camera/gaze deltas in single- and two-eye modes.
- Semantic input tests feed normalized `gaze_delta`, `view_delta`, and `reset_pose` events at the renderer/session boundary. They verify sensitivity, directions, yaw wrapping, ±89-degree pitch clamps, persistence after release, shared application to both flows, and reset after media replacement.
- Platform adapter tests remain thin. They verify the mouse mapping and double-click reset, one- versus two-finger touch mapping and double-tap reset, preview gesture ownership, overlay interception, and ordinary scrolling for gestures starting outside the preview.
- Web tests verify Pointer Events and pointer capture behavior, `touch-action: none`, browser fullscreen state synchronization, best-effort orientation lock/unlock, hidden fullscreen controls, and absence of click-to-exit behavior.
- Android tests follow existing session-state unit tests and Compose user-action tests. They verify Back-to-exit, temporary landscape only for Both, gesture routing, hidden fullscreen controls, and the rotation-only frame-size regression.
- iOS tests verify exact one- and two-touch recognizers, double-tap reset, gesture precedence around overlay controls, native fullscreen exit route, and best-effort scene-orientation requests.
- Desktop tests cover default and content-sensitive eye modes, Tab menu visibility, Escape fullscreen semantics, viewport recomputation when the menu changes, runtime file opening, and preservation of menu/fullscreen/layer state across successful and failed source changes.
- OpenXR regression tests verify that runtime view count and viewports remain authoritative and that structured eye-specific preset configuration reaches the appropriate XR eye flows without exposing the non-XR selector.
- Accessibility tests verify localized mode names, meaningful selected state beyond color, button-size semantics, and pointer tooltips. The icon silhouette itself is decorative.
- Manual visual QA covers long German labels in tooltips/accessibility output, icon recognition in light and dark themes, accidental gallery scrolling, portrait and landscape sources, RGBD media, 360-degree video, split-screen Cardboard comfort, fullscreen transitions, and rapid mode switching during video playback.

## Out of Scope

- Head tracking, device-motion camera control, or gaze tracking.
- IPD configuration, binocular camera offsets, stereo parallax, convergence, or calibrated stereoscopy.
- Cardboard or headset lens distortion, chromatic correction, lens profiles, or viewer calibration.
- A vertical/top-bottom split, adaptive split orientation, draggable divider, gutter, mask, or per-eye viewport sizing.
- Reducing per-eye internal render resolution, dynamic resolution, or a new performance/quality selector.
- Persisting eye mode, camera offsets, gaze offsets, presets, or manual session state across a full app restart.
- Displaying Left/Both/Right labels or ownership badges on every article demonstration.
- Changing eye mode from inside fullscreen on mobile or web.
- Keyboard camera or gaze navigation.
- Replacing platform-native gesture handling with Hammer.js or a cross-platform raw-touch state machine.
- Changing OpenXR view layout, introducing the non-XR split into OpenXR, or overriding runtime projection behavior.
- A broader media-library redesign beyond runtime Open File/Choose Media and transactional source replacement.

## Further Notes

- This document is the implementation source of truth for the two-eye feature. It is stored under `docs/` by explicit project direction even though older repository specifications live under `doc/`.
- The agreed test seams are the compiled catalog contract, pure session transitions, the render/display composition boundary, and thin platform adapters. These seams were confirmed during the design discussion; no additional interview is required before implementation.
- The existing strabismus content is a critical acceptance fixture because it already authors opposing left- and right-eye values and exposes the current flat-compiler data loss.
- The Android frame bridge has a known orientation hazard: its output object can swap dimensions at 90/270 degrees while pending-size tracking remains raw. The implementation must treat rotation changes as renegotiation events rather than relying only on raw width and height.
- Product copy that currently says tapping/clicking the preview exits fullscreen must be removed or updated when preview gestures become interactive.
