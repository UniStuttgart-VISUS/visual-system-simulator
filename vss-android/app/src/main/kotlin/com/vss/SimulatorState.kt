package com.vss

import android.net.Uri
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.lifecycle.ViewModel
import org.json.JSONObject

class SimulatorViewModel : ViewModel() {
    var fullscreen by mutableStateOf(false)
    var cameraDenied by mutableStateOf(false)
    var sourceUri by mutableStateOf<Uri?>(null)
    var sourceMime by mutableStateOf<String?>(null)
    var simulator by mutableStateOf(SimulatorSession())
    var articleId by mutableStateOf<String?>(null)
    var viewedArticleId by mutableStateOf<String?>(null)
    val expanded = mutableStateMapOf<String, Boolean>()
}

data class UiCatalog(val groups: List<UiGroup>, val presets: List<UiPreset>, val articles: List<UiArticle>)
data class UiGroup(val id: String, val title: String, val settings: List<UiSetting>)
data class UiChoice(val value: Int, val label: String)
data class UiSetting(val id: String, val label: String, val kind: String, val default: Any, val step: Double, val min: Double?, val max: Double?, val unit: String?, val choices: List<UiChoice>)
data class UiPreset(
    val id: String,
    val label: String,
    val both: Map<String, Any>,
    val left: Map<String, Any>,
    val right: Map<String, Any>,
)
data class UiArticle(val id: String, val title: String, val summary: String?, val image: String?, val contentPath: String, val demonstrations: List<UiDemonstration>)
data class UiDemonstration(val id: String, val label: String, val presets: Set<String>)

enum class EyeMode { LEFT, BOTH, RIGHT }

data class SessionLayer(
    val selectedDemonstrations: Map<String, String> = emptyMap(),
    val manual: Map<String, Any> = emptyMap(),
    val maskedFallback: Map<String, Any> = emptyMap(),
    val maskedByBoth: Set<String> = emptySet(),
)

data class SimulatorSession(
    val eyeMode: EyeMode = EyeMode.LEFT,
    val layers: Map<EyeMode, SessionLayer> = EyeMode.entries.associateWith { SessionLayer() },
) {
    fun withEyeMode(mode: EyeMode) = copy(eyeMode = mode)
    fun currentLayer() = layer(eyeMode)
    fun selectedDemonstration(articleId: String): String? = when (eyeMode) {
        EyeMode.LEFT, EyeMode.RIGHT -> currentLayer().selectedDemonstrations[articleId]
        EyeMode.BOTH -> layer(EyeMode.BOTH).selectedDemonstrations[articleId]
            ?: layer(EyeMode.LEFT).selectedDemonstrations[articleId]
            ?: layer(EyeMode.RIGHT).selectedDemonstrations[articleId]
    }

    fun activePresets(catalog: UiCatalog): Set<String> {
        if (eyeMode != EyeMode.BOTH) return layerPresets(currentLayer(), catalog)
        val shared = layerPresets(layer(EyeMode.BOTH), catalog)
        val intrinsic = (layerPresets(layer(EyeMode.LEFT), catalog) + layerPresets(layer(EyeMode.RIGHT), catalog))
            .filter { catalog.preset(it).isIntrinsic() }
        return shared + intrinsic
    }

    fun selectDemonstration(catalog: UiCatalog, articleId: String, demonstrationId: String): SimulatorSession {
        val article = catalog.articles.firstOrNull { it.id == articleId } ?: return this
        val selected = article.demonstrations.firstOrNull { it.id == demonstrationId } ?: return this
        val presets = selected.presets.map(catalog::preset)
        val intrinsic = presets.any(UiPreset::isIntrinsic)
        val targets = if (intrinsic) listOf(EyeMode.LEFT, EyeMode.RIGHT).filter { target ->
            presets.any { it.values(target).isNotEmpty() }
        } else listOf(eyeMode)
        val enabled = !targets.all { layer(it).selectedDemonstrations[articleId] == demonstrationId }
        var next = this
        targets.forEach { target ->
            next = next.withLayer(target, transitionLayer(next.layer(target), catalog, target, articleId, selected, enabled))
            if (target != EyeMode.BOTH && enabled) next = next.mask(listOf(target), demonstrationValues(catalog, selected, target).keys, false)
        }
        if (EyeMode.BOTH in targets) {
            val changed = demonstrationValues(catalog, selected, EyeMode.BOTH).keys
            next = if (enabled) next.mask(listOf(EyeMode.LEFT, EyeMode.RIGHT), changed, true)
            else next.mask(listOf(EyeMode.LEFT, EyeMode.RIGHT), changed.filterNot { it in next.sharedSettings(catalog) }, false)
        }
        return if (intrinsic && enabled) next.copy(eyeMode = EyeMode.BOTH) else next
    }

    fun edit(settingId: String, value: Any): SimulatorSession {
        val target = eyeMode
        var next = withLayer(target, currentLayer().copy(manual = currentLayer().manual + (settingId to value)))
        next = if (target == EyeMode.BOTH) next.mask(listOf(EyeMode.LEFT, EyeMode.RIGHT), listOf(settingId), true)
        else next.mask(listOf(target), listOf(settingId), false)
        return next
    }

    fun reset(settingId: String, catalog: UiCatalog? = null): SimulatorSession {
        var next = withLayer(eyeMode, currentLayer().copy(manual = currentLayer().manual - settingId))
        if (eyeMode == EyeMode.BOTH && (catalog == null || settingId !in next.sharedSettings(catalog))) {
            next = next.mask(listOf(EyeMode.LEFT, EyeMode.RIGHT), listOf(settingId), false)
        }
        return next
    }

    fun effectiveValues(catalog: UiCatalog, eye: EyeMode): Map<String, Any> {
        require(eye != EyeMode.BOTH)
        val result = catalog.groups.flatMap { it.settings }.associate { it.id to it.default }.toMutableMap()
        applyLayer(result, layer(EyeMode.BOTH), catalog, EyeMode.BOTH)
        applyLayer(result, layer(eye), catalog, eye)
        return result
    }

    fun editableValues(catalog: UiCatalog): Map<String, Any> {
        if (eyeMode != EyeMode.BOTH) return effectiveValues(catalog, eyeMode)
        val result = catalog.groups.flatMap { it.settings }.associate { it.id to it.default }.toMutableMap()
        applyLayer(result, layer(EyeMode.BOTH), catalog, EyeMode.BOTH)
        return result
    }

    fun sourceArticle(catalog: UiCatalog, settingId: String): String? {
        val layer = currentLayer()
        if (settingId in layer.manual) return null
        val owner = catalog.presets.firstOrNull { it.id in layerPresets(layer, catalog) && settingId in it.values(eyeMode) } ?: return null
        return catalog.articles.firstOrNull { article -> article.demonstrations.any { owner.id in it.presets } }?.id
    }

    private fun layer(mode: EyeMode) = layers.getValue(mode)
    private fun withLayer(mode: EyeMode, layer: SessionLayer) = copy(layers = layers + (mode to layer))
    private fun mask(targets: List<EyeMode>, settings: Iterable<String>, masked: Boolean): SimulatorSession {
        var next = this
        targets.forEach { target ->
            val layer = next.layer(target)
            next = next.withLayer(target, layer.copy(maskedByBoth = if (masked) layer.maskedByBoth + settings else layer.maskedByBoth - settings.toSet()))
        }
        return next
    }
    private fun sharedSettings(catalog: UiCatalog) = layer(EyeMode.BOTH).manual.keys + settingsForPresets(catalog, EyeMode.BOTH, layerPresets(layer(EyeMode.BOTH), catalog))
}

private fun UiCatalog.preset(id: String) = presets.first { it.id == id }
private fun UiPreset.isIntrinsic() = left.isNotEmpty() || right.isNotEmpty()
private fun UiPreset.values(target: EyeMode) = if (both.isNotEmpty()) both else when (target) { EyeMode.LEFT -> left; EyeMode.RIGHT -> right; EyeMode.BOTH -> emptyMap() }
private fun layerPresets(layer: SessionLayer, catalog: UiCatalog) = catalog.articles.flatMap { article ->
    article.demonstrations.firstOrNull { it.id == layer.selectedDemonstrations[article.id] }?.presets.orEmpty()
}.toSet()
private fun demonstrationValues(catalog: UiCatalog, demo: UiDemonstration, target: EyeMode) = demo.presets.flatMap { catalog.preset(it).values(target).entries }.associate { it.key to it.value }
private fun transitionLayer(layer: SessionLayer, catalog: UiCatalog, target: EyeMode, articleId: String, selected: UiDemonstration, enabled: Boolean): SessionLayer {
    val before = layerPresets(layer, catalog)
    val selections = layer.selectedDemonstrations.toMutableMap()
    if (!enabled) selections.remove(articleId) else {
        val values = demonstrationValues(catalog, selected, target)
        catalog.articles.forEach { article ->
            val other = article.demonstrations.firstOrNull { it.id == selections[article.id] }
            if (article.id != articleId && other != null && demonstrationValues(catalog, other, target).any { (key, value) -> values[key]?.let { it != value } == true }) selections.remove(article.id)
        }
        selections[articleId] = selected.id
    }
    return cascadePresetTransition(catalog, target, before, layerPresets(layer.copy(selectedDemonstrations = selections), catalog), layer.copy(selectedDemonstrations = selections))
}
private fun settingsForPresets(catalog: UiCatalog, target: EyeMode, presets: Set<String>) = presets.flatMap { catalog.preset(it).values(target).keys }.toSet()
private fun cascadePresetTransition(catalog: UiCatalog, target: EyeMode, before: Set<String>, after: Set<String>, layer: SessionLayer): SessionLayer {
    val beforeSettings = settingsForPresets(catalog, target, before); val afterSettings = settingsForPresets(catalog, target, after)
    val manual = layer.manual.toMutableMap(); val fallback = layer.maskedFallback.toMutableMap()
    (beforeSettings + afterSettings).forEach { setting ->
        if (setting in afterSettings) manual.remove(setting)?.let { fallback[setting] = it }
        else if (setting in beforeSettings) { if (setting in manual) fallback.remove(setting) else fallback.remove(setting)?.let { manual[setting] = it } }
    }
    return layer.copy(manual = manual, maskedFallback = fallback)
}
private fun applyLayer(result: MutableMap<String, Any>, layer: SessionLayer, catalog: UiCatalog, target: EyeMode) {
    catalog.presets.filter { it.id in layerPresets(layer, catalog) }.forEach { preset -> preset.values(target).filterKeys { it !in layer.maskedByBoth }.forEach(result::put) }
    layer.manual.filterKeys { it !in layer.maskedByBoth }.forEach(result::put)
}

fun parseCatalog(json: String): UiCatalog {
    val root = JSONObject(json)
    val groups = root.getJSONArray("groups").let { groups ->
        (0 until groups.length()).map { i ->
            val group = groups.getJSONObject(i)
            val settings = group.getJSONArray("settings")
            UiGroup(group.getString("id"), group.getString("title"), (0 until settings.length()).map { j ->
                val setting = settings.getJSONObject(j)
                val control = setting.getJSONObject("control")
                val choices = control.optJSONArray("choices")
                UiSetting(
                    setting.getString("id"), setting.getString("label"), control.getString("kind"),
                    setting.get("default"), control.optDouble("step", 1.0),
                    control.optDouble("min").takeUnless { control.isNull("min") || it.isNaN() },
                    control.optDouble("max").takeUnless { control.isNull("max") || it.isNaN() },
                    setting.optString("unit").takeIf { it.isNotEmpty() && it != "null" },
                    if (choices == null) emptyList() else (0 until choices.length()).map { k ->
                        choices.getJSONObject(k).let { UiChoice(it.getInt("value"), it.getString("label")) }
                    },
                )
            })
        }
    }
    val presets = root.getJSONArray("presets").let { presets ->
        (0 until presets.length()).map { i ->
            val preset = presets.getJSONObject(i)
            fun section(name: String) = preset.getJSONObject(name).toMap()
            UiPreset(
                preset.getString("id"),
                preset.getString("label"),
                section("both"),
                section("left"),
                section("right"),
            )
        }
    }
    val articles = root.getJSONArray("articles").let { articles ->
        (0 until articles.length()).map { i ->
            val article = articles.getJSONObject(i)
            val demonstrations = article.getJSONArray("demonstrations")
            UiArticle(article.getString("id"), article.getString("title"), article.optString("summary").takeIf { it.isNotEmpty() && it != "null" }, article.optString("image").takeIf { it.isNotEmpty() && it != "null" }, article.getString("content_path"), (0 until demonstrations.length()).map { j ->
                demonstrations.getJSONObject(j).let { demo ->
                    UiDemonstration(demo.getString("id"), demo.getString("label"), demo.getJSONArray("presets").let { presets ->
                        (0 until presets.length()).map(presets::getString).toSet()
                    })
                }
            })
        }
    }
    return UiCatalog(groups, presets, articles)
}

fun JSONObject.toMap(): Map<String, Any> = keys().asSequence().associateWith(::get)
