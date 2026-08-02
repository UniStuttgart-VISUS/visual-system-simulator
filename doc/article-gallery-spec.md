# Article Discovery and Preset Context

## Problem Statement

The simulator's information architecture is sound: the live camera is the primary experience, articles explain visual conditions, presets provide quick access to simulations, and settings provide fine tuning. The current presentation does not communicate that structure. Articles and presets appear as separate rows of small chips, so both look like filters or commands. Articles do not invite exploration, presets do not visibly belong to a knowledge topic, and users cannot easily understand which article explains a preset-derived setting value.

The shared catalog also lacks an explicit editorial article order and card-ready metadata. Clients currently receive articles in an implementation-derived order. This makes it difficult to present the same knowledge structure coherently on Android now and on iOS and Web later.

## Solution

Present articles as a horizontally snapping discovery gallery. Each card represents one article and combines a stable editorial image, a short canonical title overlaid on the image, an information action beside the title, and the article's demonstrations as a segmented control attached directly to the bottom edge. The image, title, and information action open the article; the demonstration segments apply or remove presets. Articles without demonstrations remain complete knowledge cards and reserve the same bottom area as actionable cards without showing disabled or placeholder controls.

Keep the live camera as the result surface and place the gallery before the settings in the same vertical scroll container. A compact passive index immediately above the gallery communicates article position and which articles have active presets. The index uses shape and thickness in addition to color.

Make preset provenance available during fine tuning. A setting whose effective value comes from an active preset shows an information action that opens the owning article. If the user manually overrides that setting, the same action slot instead shows reset. Reset removes the current manual override and reveals the inherited preset or default value.

Presets temporarily mask pre-existing manual values rather than destroying them. During the current app session, disabling the last relevant preset restores the previous manual value unless the user supplied a newer manual value while the preset was active. The app does not persist simulator state across full restarts, so this masked state is session-only.

The shared catalog remains presentation-neutral. It supplies ordered articles and neutral article metadata; Android initially renders the gallery. iOS and Web may adopt the same design later without requiring another content-model redesign.

## User Stories

1. As a simulator user, I want the live camera to remain the primary surface, so that I immediately see the result of a simulation.
2. As a curious user, I want articles to look like explorable editorial content, so that I am invited to learn rather than merely operate filters.
3. As a user, I want each card to represent one coherent visual-system topic, so that the knowledge structure is understandable.
4. As a user, I want a card's image and overlaid title to open its article, so that the article has a large and obvious touch target.
5. As a user, I want an information button beside the overlaid title, so that the card's educational purpose and open action are explicit.
6. As a user, I want related demonstrations attached to their article card, so that I understand why those presets exist.
7. As a user, I want related variants such as weak and strong cataract grouped in one segmented control, so that they are understood as alternatives of one topic.
8. As a user, I want selecting one variant to replace another variant of the same article, so that contradictory variants cannot be active together.
9. As a user, I want tapping the selected variant again to disable it, so that turning a simulation off is direct.
10. As a user, I want compatible presets from different articles to combine, so that I can explore layered visual conditions.
11. As a user, I want presets that assign different values to the same setting to replace one another, so that invisible precedence cannot produce misleading results.
12. As a user, I want articles without demonstrations to remain discoverable, so that simulator limitations do not limit education.
13. As a user, I do not want unavailable demonstrations represented by disabled controls or "coming soon" labels, so that knowledge-only cards still feel complete.
14. As a user, I want a preview of the next card to remain visible, so that horizontal exploration is discoverable.
15. As a user, I want cards to snap to a stable article position, so that the gallery feels controlled and readable.
16. As a user, I want article order to remain stable while I interact, so that the interface does not move beneath me.
17. As a returning user within the same session, I want the gallery to remember the last viewed article, so that I can continue where I was.
18. As a user opening a fresh gallery with active presets, I want it initially focused on the first active article in editorial order, so that the content is context-sensitive without later auto-scrolling.
19. As a user with no active presets, I want the gallery to begin with the first editorially curated article, so that discovery has an intentional entry point.
20. As a user, I want a compact index to show my position in the article collection, so that I can judge how much content remains.
21. As a user, I want the index to show which articles have active presets, so that active simulations remain visible even when their cards are off-screen.
22. As a user with color-vision or low-vision needs, I want index states distinguished by shape and thickness as well as color, so that status does not depend on color perception.
23. As a touch user, I do not want the thin index to act as a hidden touch control, so that accidental navigation is avoided.
24. As a screen-reader user, I want each card announced with its title, position, active variant, and available actions, so that the gallery is navigable without relying on the decorative image.
25. As a screen-reader user, I do not want the decorative card image announced separately, so that repeated information does not slow navigation.
26. As a user, I want the card image to remain stable when I change variants, so that the gallery preserves editorial context while the live camera shows the actual result.
27. As a user, I want multi-form topics such as ametropia represented by one shared article card, so that myopia and hyperopia remain part of the same knowledge structure.
28. As a user, I want multi-form cards to use a comparison image, so that distinct visual forms are understandable without duplicating an article.
29. As a user, I want the gallery to scroll vertically away with the settings, so that it can be inviting without permanently reducing fine-tuning space.
30. As a reader, I want long articles to open in a nearly full-height sheet, so that they are comfortable to read while remaining connected to the simulator.
31. As a reader, I want closing an article to return me to the same horizontal and vertical position, so that I do not lose context.
32. As a reader, I want the article's demonstrations available in a fixed segmented control in the article sheet, so that I can move directly from learning to experiencing.
33. As a reader, I want choosing a demonstration in the article to close the sheet and reveal the live result, so that feedback is immediate.
34. As a user, I do not want an additional replace-versus-add dialog, so that applying a demonstration remains direct.
35. As a user fine-tuning a preset, I want a preset-derived setting to show an information action, so that I can understand the source and rationale for the value.
36. As a user fine-tuning a preset, I want a manually overridden setting to show reset instead of information in the same action slot, so that the row remains compact and the primary action is clear.
37. As a user, I want resetting a manual override to reveal the inherited preset value when a relevant preset is active, so that reset has a consistent meaning.
38. As a user, I want resetting a manual value without a relevant preset to reveal the default, so that reset has a consistent meaning.
39. As a user, I want a manual value that existed before a preset to be restored when that preset is disabled, so that exploring presets does not destroy my work.
40. As a user, I want a newer manual value entered over an active preset to supersede the older masked value, so that my latest explicit choice wins.
41. As a user, I want masked values to live only for the current session, so that the behavior matches the app's existing reset-on-restart lifecycle.
42. As a content editor, I want article order defined explicitly, so that it is not determined by identifiers, file paths, or client code.
43. As a content editor, I want every article included exactly once in the index, so that new or orphaned content cannot silently disappear.
44. As a content editor, I want one canonical title and one demonstration label per locale, so that the catalog does not accumulate long/short naming synonyms.
45. As a content editor, I want short canonical titles, so that the same title works in gallery cards and article sheets.
46. As a content editor, I want concise demonstration labels such as "weak" and "strong", so that the same labels fit the attached segmented control.
47. As a content editor, I want one 16:9 article image reference, so that clients can build stable cards without unpredictable cropping.
48. As a catalog maintainer, I want each preset to belong unambiguously to one article, so that preset provenance can always open the correct knowledge source.
49. As an iOS or Web implementer, I want presentation-neutral catalog metadata, so that later clients can adopt the design without Android concepts in the shared contract.

## Implementation Decisions

- The article collection has an explicit index containing an ordered list named `articles`. The index must contain every article identifier exactly once and must not contain unknown or duplicate identifiers.
- Initial editorial order is: ametropia, presbyopia, cataract, color deficiency, achromatopsia, nyctalopia, glaucoma, and macular degeneration.
- The shared article contract gains `image` metadata. It may remain optional during content migration; Android must render a useful card when it is absent. It should become required for indexed articles before the gallery is considered content-complete.
- The card image uses a curated 16:9 source, remains visually static as variants change, is decorative in the gallery, and is excluded from the accessibility tree. Article-body images retain their separate Markdown accessibility responsibilities.
- Existing article titles become short canonical titles. Parenthetical aliases and explanatory wording move to the article introduction rather than a second title field.
- Existing demonstration labels become compact localized variant labels. No second short-label field is introduced.
- Every preset must be referenced by at least one demonstration, and all demonstrations referencing a preset must belong to the same article. This creates an unambiguous preset-to-article relationship without adding article references to settings or groups.
- A gallery card represents an article, not a preset. Its title is overlaid on the bottom of the image with sufficient contrast, with an information button beside it. The image, title, and information button open the article; only demonstration segments mutate simulation state.
- Every card reserves an equal-height bottom area. Demonstrations are rendered there as one attached segmented control. An article with no demonstrations leaves that area empty and has no unavailable-state messaging.
- When a localized demonstration label equals its article title, Android replaces the repeated segment text with the action label `Activate` or `Deactivate`, according to the current selection state.
- The gallery uses horizontal snapping with one primary card and visible neighboring peeks inside the narrow catalog panel. The responsive outer layout remains responsible for placing the camera and catalog panel.
- The passive index, gallery, and settings appear in that order and share one vertical scroll container. The gallery is not sticky.
- The passive index has one segment per article, never one per preset. It reflects the nearest snapped article and active article presets. Current and active states differ in geometry and weight as well as color. The index is not interactive.
- Gallery ordering never changes at runtime. Within a session, the last viewed article is restored. On a fresh state, the first active article in index order is selected, otherwise the first indexed article is selected. The UI does not automatically move after initial selection.
- Selecting a demonstration activates all presets it references. Selecting another demonstration from the same article replaces the prior selection. Selecting the already selected demonstration removes it.
- Demonstrations from different articles may coexist when their presets do not assign different values to the same setting. Selecting a conflicting demonstration removes the conflicting active demonstration without prompting.
- The Android state model distinguishes pre-preset manual values, active preset values, and post-preset manual overrides on a per-setting basis. This is a current-state cascade, not an undo history.
- When a preset first masks a manual value, that value is retained as the session fallback. A manual edit while the preset is active becomes the newest explicit user value. When the last relevant preset is removed, the newest manual value remains if present; otherwise the masked fallback is restored.
- Switching to a conflicting or alternative demonstration keeps the same masked fallback unless a newer manual value has replaced it.
- The manual-layer cascade is not written to disk and resets with the rest of the app on a full restart.
- A setting row has one contextual action slot. A current manual override shows reset. Otherwise, a value supplied by an active preset shows information linked to that preset's owning article. A default-derived setting shows no action.
- Reset always removes the currently effective manual contribution. It reveals a preset value when one exists and otherwise reveals the default.
- Article content opens in an immediately expanded, nearly full-height modal sheet. It restores the caller's scroll context on dismissal.
- The article sheet repeats the same demonstration control in a fixed bottom area. Selecting a demonstration applies it, dismisses the sheet, and reveals the live camera. The existing replace/add decision dialog is removed from this flow.
- Android is the initial presentation implementation. The shared catalog contract and article order are available to all clients, but iOS and Web retain their current UIs until separately changed.
- The catalog vocabulary remains `Article`, `Demonstration`, `Preset`, and `Setting`. Presentation terms such as gallery, card, and topic do not enter the shared schema.

## Testing Decisions

- Tests should assert externally visible state transitions and serialized contract behavior rather than private Compose structure, specific composable names, or pixel coordinates.
- The highest shared test seam is the serialized catalog contract. Contract tests should verify exact article order, index completeness, unique identifiers, optional image metadata serialization, short-label localization, demonstration references, and unique preset ownership.
- Existing catalog consistency tests are the prior art for extending contract validation. They already validate unique setting and preset identifiers, preset values, localized fallback behavior, article compilation, and demonstration references.
- Catalog tests should verify that unknown, duplicate, or missing article index entries fail the build or validation step with actionable diagnostics.
- Catalog tests should verify that a preset referenced from two different articles is rejected and that repeated use within the same article remains valid.
- Catalog tests should verify conflict detection using shared setting identifiers and differing values, while allowing shared identical assignments.
- The highest Android state seam should be a pure state transition API beneath Compose. Tests should cover activating, replacing, combining, and disabling demonstrations; per-setting conflict replacement; masked manual fallback; newer-manual-value precedence; reset behavior with and without a preset; and article provenance for effective settings.
- Android state tests should explicitly cover the sequence: manual value, preset activation, preset deactivation, and restoration of the manual value.
- Android state tests should explicitly cover the sequence: manual value, preset activation, newer manual value, preset deactivation, and retention of the newer value.
- Android state tests should cover reset while a preset is active and reset while no preset is active.
- Android state tests should cover a demonstration containing multiple presets even if current content generally uses one preset per demonstration.
- Compose tests should operate at the user-action seam: tap a card to open its article, tap a segment to activate it, tap it again to disable it, switch variants, dismiss the article, and verify contextual setting actions.
- Compose semantics tests should verify card position announcements, active variant announcements, useful touch targets, decorative image exclusion, and index state descriptions without relying on color.
- UI tests should verify that articles without demonstrations have no disabled action footer and still open normally.
- UI tests should verify that article dismissal preserves gallery and settings scroll context.
- A manual visual QA pass should verify snap feel, next-card peek, fixed card height, 16:9 cropping, long German labels, dark-theme contrast, index legibility, and the transition between information and reset actions.

## Out of Scope

- Creating final gallery images. The image field and fallback are included, but content production may follow separately.
- Redesigning article body content, citations, typography, or Markdown image accessibility beyond the new article-sheet container.
- Implementing the gallery presentation on iOS or Web.
- Persisting simulator, gallery, masked-manual, or scroll state across a full app restart.
- Adding a user-reorderable preset stack or exposing preset precedence controls.
- Making the passive article index tappable.
- Automatically reordering the gallery in response to active presets or manual settings.
- Adding article references directly to setting or group definitions.
- Adding unavailable, disabled, or "coming soon" demonstration placeholders.
- Publishing or hosting the new article images.

## Further Notes

- The article index is stored as `articles/index.toml` within the shared catalog and uses a single top-level `articles` array. The filename and field carry the semantics; no redundant `gallery`, `topics`, or `index` table is introduced.
- The agreed initial German card vocabulary uses concise canonical terms such as Ametropie, Presbyopie, Katarakt, Farbfehlsichtigkeit, Achromatopsie, Nachtblindheit, Glaukom, and Makuladegeneration.
- The gallery image and canonical title are stable editorial context. The live camera remains the authoritative visualization of the currently active simulation.
- This specification could not be published to the project issue tracker because no issue-tracker integration or GitHub CLI is available in the current environment. The repository document is the source artifact until it is copied into an issue and labeled `ready-for-agent`.
