package com.vss

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class SimulatorSessionTest {
    private fun session() = SimulatorSession().withEyeMode(EyeMode.BOTH)

    private val catalog = UiCatalog(
        groups = emptyList(),
        presets = listOf(
            UiPreset("weak", "Weak", mapOf("blur" to 25.0, "enabled" to true), emptyMap(), emptyMap()),
            UiPreset("strong", "Strong", mapOf("blur" to 75.0, "enabled" to true), emptyMap(), emptyMap()),
            UiPreset("same", "Same", mapOf("enabled" to true), emptyMap(), emptyMap()),
            UiPreset("conflict", "Conflict", mapOf("blur" to 40.0), emptyMap(), emptyMap()),
        ),
        articles = listOf(
            UiArticle("cataract", "Cataract", null, null, "cataract.html", listOf(
                UiDemonstration("weak", "Weak", setOf("weak")),
                UiDemonstration("strong", "Strong", setOf("strong")),
            )),
            UiArticle("other", "Other", null, null, "other.html", listOf(
                UiDemonstration("same", "Same", setOf("same")),
                UiDemonstration("multi", "Multi", setOf("same", "conflict")),
            )),
        ),
    )

    @Test fun intrinsic_preset_selects_both_then_symmetric_preset_targets_visible_left_eye() {
        val eyeCatalog = UiCatalog(
            emptyList(),
            listOf(
                UiPreset("strabismus", "Strabismus", emptyMap(), mapOf("axis" to .05), mapOf("axis" to -.05)),
                UiPreset("cataract", "Cataract", mapOf("blur" to 25.0), emptyMap(), emptyMap()),
            ),
            listOf(
                UiArticle("strabismus", "Strabismus", null, null, "", listOf(UiDemonstration("strabismus", "Strabismus", setOf("strabismus")))),
                UiArticle("cataract", "Cataract", null, null, "", listOf(UiDemonstration("cataract", "Cataract", setOf("cataract")))),
            ),
        )
        var state = SimulatorSession()
        assertEquals(EyeMode.LEFT, state.eyeMode)
        state = state.selectDemonstration(eyeCatalog, "strabismus", "strabismus")
        assertEquals(EyeMode.BOTH, state.eyeMode)
        assertEquals(.05, state.effectiveValues(eyeCatalog, EyeMode.LEFT)["axis"])
        assertEquals(-.05, state.effectiveValues(eyeCatalog, EyeMode.RIGHT)["axis"])
        state = state.withEyeMode(EyeMode.LEFT).selectDemonstration(eyeCatalog, "cataract", "cataract")
        assertEquals(25.0, state.effectiveValues(eyeCatalog, EyeMode.LEFT)["blur"])
        assertNull(state.effectiveValues(eyeCatalog, EyeMode.RIGHT)["blur"])
        assertEquals(-.05, state.effectiveValues(eyeCatalog, EyeMode.RIGHT)["axis"])
    }

    @Test fun manual_value_is_restored_after_the_last_relevant_preset_is_disabled() {
        val manual = session().edit("blur", 10.0)
        val active = manual.selectDemonstration(catalog, "cataract", "weak")
        assertEquals(emptyMap<String, Any>(), active.currentLayer().manual)
        assertEquals(10.0, active.currentLayer().maskedFallback["blur"])
        val disabled = active.selectDemonstration(catalog, "cataract", "weak")
        assertEquals(10.0, disabled.currentLayer().manual["blur"])
    }

    @Test fun newest_manual_value_wins_when_a_preset_is_disabled() {
        val state = session().edit("blur", 10.0)
            .selectDemonstration(catalog, "cataract", "weak")
            .edit("blur", 15.0)
            .selectDemonstration(catalog, "cataract", "weak")
        assertEquals(15.0, state.currentLayer().manual["blur"])
        assertNull(state.currentLayer().maskedFallback["blur"])
    }

    @Test fun switching_variants_replaces_the_old_variant_and_masks_the_latest_manual_value() {
        val state = session().selectDemonstration(catalog, "cataract", "weak")
            .edit("blur", 15.0)
            .selectDemonstration(catalog, "cataract", "strong")
        assertEquals(setOf("strong"), state.activePresets(catalog))
        assertEquals(15.0, state.currentLayer().maskedFallback["blur"])
        assertNull(state.currentLayer().manual["blur"])
    }

    @Test fun identical_assignments_combine_and_differing_assignments_replace_without_a_dialog() {
        val combined = session().selectDemonstration(catalog, "cataract", "weak")
            .selectDemonstration(catalog, "other", "same")
        assertEquals(setOf("weak", "same"), combined.activePresets(catalog))
        val replaced = combined.selectDemonstration(catalog, "cataract", "strong")
        assertEquals(setOf("strong", "same"), replaced.activePresets(catalog))
        val conflict = replaced.selectDemonstration(catalog, "other", "multi")
        assertEquals(setOf("same", "conflict"), conflict.activePresets(catalog))
        assertNull(conflict.currentLayer().selectedDemonstrations["cataract"])
    }

    @Test fun reset_reveals_preset_and_provenance_then_default_without_a_preset() {
        val active = session().selectDemonstration(catalog, "cataract", "weak").edit("blur", 12.0)
        assertNull(active.sourceArticle(catalog, "blur"))
        val reset = active.reset("blur", catalog)
        assertEquals("cataract", reset.sourceArticle(catalog, "blur"))
        val disabled = reset.selectDemonstration(catalog, "cataract", "weak")
        assertNull(disabled.sourceArticle(catalog, "blur"))
    }
}
