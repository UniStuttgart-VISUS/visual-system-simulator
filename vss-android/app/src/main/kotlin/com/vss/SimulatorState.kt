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
    var activePresets by mutableStateOf(setOf<String>())
    var overrides by mutableStateOf<Map<String, Any>>(emptyMap())
    var articleId by mutableStateOf<String?>(null)
    var pendingPresetRemoval by mutableStateOf<String?>(null)
    var pendingDemoPresets by mutableStateOf<Set<String>?>(null)
    val expanded = mutableStateMapOf<String, Boolean>()
}

data class UiCatalog(val groups: List<UiGroup>, val presets: List<UiPreset>, val articles: List<UiArticle>)
data class UiGroup(val id: String, val title: String, val settings: List<UiSetting>)
data class UiChoice(val value: Int, val label: String)
data class UiSetting(val id: String, val label: String, val kind: String, val default: Any, val step: Double, val min: Double?, val max: Double?, val unit: String?, val choices: List<UiChoice>)
data class UiPreset(val id: String, val label: String, val values: Map<String, Any>)
data class UiArticle(val id: String, val title: String, val contentPath: String, val demonstrations: List<Pair<String, Set<String>>>)

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
            UiArticle(article.getString("id"), article.getString("title"), article.getString("content_path"), (0 until demonstrations.length()).map { j ->
                demonstrations.getJSONObject(j).let { demo ->
                    demo.getString("label") to demo.getJSONArray("presets").let { presets ->
                        (0 until presets.length()).map(presets::getString).toSet()
                    }
                }
            })
        }
    }
    return UiCatalog(groups, presets, articles)
}

fun JSONObject.toMap(): Map<String, Any> = keys().asSequence().associateWith(::get)
