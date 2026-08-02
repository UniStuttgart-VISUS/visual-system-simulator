package com.vss

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class SimulatorSessionTest {
    private val catalog = UiCatalog(
        groups = emptyList(),
        presets = listOf(
            UiPreset("weak", "Weak", mapOf("blur" to 25.0, "enabled" to true)),
            UiPreset("strong", "Strong", mapOf("blur" to 75.0, "enabled" to true)),
            UiPreset("same", "Same", mapOf("enabled" to true)),
            UiPreset("conflict", "Conflict", mapOf("blur" to 40.0)),
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

    @Test fun manual_value_is_restored_after_the_last_relevant_preset_is_disabled() {
        val manual = SimulatorSession().edit("blur", 10.0)
        val active = manual.selectDemonstration(catalog, "cataract", "weak")
        assertEquals(emptyMap<String, Any>(), active.manual)
        assertEquals(10.0, active.maskedFallback["blur"])
        val disabled = active.selectDemonstration(catalog, "cataract", "weak")
        assertEquals(10.0, disabled.manual["blur"])
    }

    @Test fun newest_manual_value_wins_when_a_preset_is_disabled() {
        val state = SimulatorSession().edit("blur", 10.0)
            .selectDemonstration(catalog, "cataract", "weak")
            .edit("blur", 15.0)
            .selectDemonstration(catalog, "cataract", "weak")
        assertEquals(15.0, state.manual["blur"])
        assertNull(state.maskedFallback["blur"])
    }

    @Test fun switching_variants_replaces_the_old_variant_and_masks_the_latest_manual_value() {
        val state = SimulatorSession().selectDemonstration(catalog, "cataract", "weak")
            .edit("blur", 15.0)
            .selectDemonstration(catalog, "cataract", "strong")
        assertEquals(setOf("strong"), state.activePresets(catalog))
        assertEquals(15.0, state.maskedFallback["blur"])
        assertNull(state.manual["blur"])
    }

    @Test fun identical_assignments_combine_and_differing_assignments_replace_without_a_dialog() {
        val combined = SimulatorSession().selectDemonstration(catalog, "cataract", "weak")
            .selectDemonstration(catalog, "other", "same")
        assertEquals(setOf("weak", "same"), combined.activePresets(catalog))
        val replaced = combined.selectDemonstration(catalog, "cataract", "strong")
        assertEquals(setOf("strong", "same"), replaced.activePresets(catalog))
        val conflict = replaced.selectDemonstration(catalog, "other", "multi")
        assertEquals(setOf("same", "conflict"), conflict.activePresets(catalog))
        assertNull(conflict.selectedDemonstrations["cataract"])
    }

    @Test fun reset_reveals_preset_and_provenance_then_default_without_a_preset() {
        val active = SimulatorSession().selectDemonstration(catalog, "cataract", "weak").edit("blur", 12.0)
        assertNull(active.sourceArticle(catalog, "blur"))
        val reset = active.reset("blur")
        assertEquals("cataract", reset.sourceArticle(catalog, "blur"))
        val disabled = reset.selectDemonstration(catalog, "cataract", "weak")
        assertNull(disabled.sourceArticle(catalog, "blur"))
    }
}
