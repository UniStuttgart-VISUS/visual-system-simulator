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
data class UiPreset(val id: String, val label: String, val values: Map<String, Any>)
data class UiArticle(val id: String, val title: String, val summary: String?, val image: String?, val contentPath: String, val demonstrations: List<UiDemonstration>)
data class UiDemonstration(val id: String, val label: String, val presets: Set<String>)

data class SimulatorSession(
    val selectedDemonstrations: Map<String, String> = emptyMap(),
    val manual: Map<String, Any> = emptyMap(),
    val maskedFallback: Map<String, Any> = emptyMap(),
) {
    fun activePresets(catalog: UiCatalog): Set<String> = catalog.articles.flatMap { article ->
        article.demonstrations.firstOrNull { it.id == selectedDemonstrations[article.id] }?.presets.orEmpty()
    }.toSet()

    fun selectDemonstration(catalog: UiCatalog, articleId: String, demonstrationId: String): SimulatorSession {
        val article = catalog.articles.first { it.id == articleId }
        val selected = article.demonstrations.first { it.id == demonstrationId }
        val beforePresets = activePresets(catalog)
        val nextSelections = selectedDemonstrations.toMutableMap()
        if (nextSelections[articleId] == demonstrationId) {
            nextSelections.remove(articleId)
        } else {
            val selectedValues = selected.presets.flatMap { id -> catalog.preset(id).values.entries }
                .associate { it.key to it.value }
            catalog.articles.forEach { otherArticle ->
                val other = otherArticle.demonstrations.firstOrNull { it.id == nextSelections[otherArticle.id] }
                if (other != null && otherArticle.id != articleId && demonstrationsConflict(catalog, selectedValues, other)) {
                    nextSelections.remove(otherArticle.id)
                }
            }
            nextSelections[articleId] = demonstrationId
        }
        val next = copy(selectedDemonstrations = nextSelections)
        return cascadePresetTransition(catalog, beforePresets, next.activePresets(catalog), next)
    }

    fun edit(settingId: String, value: Any) = copy(manual = manual + (settingId to value))
    fun reset(settingId: String) = copy(manual = manual - settingId)

    fun sourceArticle(catalog: UiCatalog, settingId: String): String? {
        if (settingId in manual) return null
        val active = activePresets(catalog)
        val preset = catalog.presets.firstOrNull { it.id in active && settingId in it.values } ?: return null
        return catalog.articles.firstOrNull { article ->
            article.demonstrations.any { preset.id in it.presets }
        }?.id
    }
}

private fun UiCatalog.preset(id: String) = presets.first { it.id == id }

private fun demonstrationsConflict(catalog: UiCatalog, selectedValues: Map<String, Any>, other: UiDemonstration): Boolean =
    other.presets.flatMap { catalog.preset(it).values.entries }.any { (setting, value) ->
        selectedValues[setting]?.let { it != value } == true
    }

private fun cascadePresetTransition(catalog: UiCatalog, before: Set<String>, after: Set<String>, state: SimulatorSession): SimulatorSession {
    val beforeSettings = before.flatMap { catalog.preset(it).values.keys }.toSet()
    val afterSettings = after.flatMap { catalog.preset(it).values.keys }.toSet()
    val manual = state.manual.toMutableMap()
    val fallback = state.maskedFallback.toMutableMap()
    (beforeSettings + afterSettings).forEach { setting ->
        when {
            setting !in beforeSettings && setting in afterSettings -> manual.remove(setting)?.let { fallback[setting] = it }
            setting in beforeSettings && setting in afterSettings -> manual.remove(setting)?.let { fallback[setting] = it }
            setting in beforeSettings && setting !in afterSettings -> {
                if (setting in manual) fallback.remove(setting) else fallback.remove(setting)?.let { manual[setting] = it }
            }
        }
    }
    return state.copy(manual = manual, maskedFallback = fallback)
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
            val values = preset.getJSONObject("values")
            UiPreset(preset.getString("id"), preset.getString("label"), values.keys().asSequence().associateWith(values::get))
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
