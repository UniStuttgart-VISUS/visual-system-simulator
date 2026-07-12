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
    var activeProfiles by mutableStateOf(setOf<String>())
    var overrides by mutableStateOf<Map<String, Any>>(emptyMap())
    var articleId by mutableStateOf<String?>(null)
    var pendingProfileRemoval by mutableStateOf<String?>(null)
    var pendingDemoProfiles by mutableStateOf<Set<String>?>(null)
    val expanded = mutableStateMapOf<String, Boolean>()
}

data class UiCatalog(val groups: List<UiGroup>, val profiles: List<UiProfile>, val articles: List<UiArticle>)
data class UiGroup(val id: String, val title: String, val parameters: List<UiParameter>)
data class UiChoice(val value: Int, val label: String)
data class UiParameter(val key: String, val label: String, val kind: String, val default: Any, val step: Double, val min: Double?, val max: Double?, val unit: String?, val choices: List<UiChoice>)
data class UiProfile(val id: String, val label: String, val values: Map<String, Any>)
data class UiArticle(val id: String, val title: String, val contentPath: String, val demonstrations: List<Pair<String, Set<String>>>)

fun parseCatalog(json: String): UiCatalog {
    val root = JSONObject(json)
    val groups = root.getJSONArray("groups").let { groups ->
        (0 until groups.length()).map { i ->
            val group = groups.getJSONObject(i)
            val parameters = group.getJSONArray("parameters")
            UiGroup(group.getString("id"), group.getString("title"), (0 until parameters.length()).map { j ->
                val parameter = parameters.getJSONObject(j)
                val control = parameter.getJSONObject("control")
                val choices = control.optJSONArray("choices")
                UiParameter(
                    parameter.getString("key"), parameter.getString("label"), control.getString("kind"),
                    parameter.get("default"), control.optDouble("step", 1.0),
                    control.optDouble("min").takeUnless { control.isNull("min") || it.isNaN() },
                    control.optDouble("max").takeUnless { control.isNull("max") || it.isNaN() },
                    parameter.optString("unit").takeIf { it.isNotEmpty() && it != "null" },
                    if (choices == null) emptyList() else (0 until choices.length()).map { k ->
                        choices.getJSONObject(k).let { UiChoice(it.getInt("value"), it.getString("label")) }
                    },
                )
            })
        }
    }
    val profiles = root.getJSONArray("profiles").let { profiles ->
        (0 until profiles.length()).map { i ->
            val profile = profiles.getJSONObject(i)
            val values = profile.getJSONObject("values")
            UiProfile(profile.getString("id"), profile.getString("label"), values.keys().asSequence().associateWith(values::get))
        }
    }
    val articles = root.getJSONArray("articles").let { articles ->
        (0 until articles.length()).map { i ->
            val article = articles.getJSONObject(i)
            val demonstrations = article.getJSONArray("demonstrations")
            UiArticle(article.getString("id"), article.getString("title"), article.getString("content_path"), (0 until demonstrations.length()).map { j ->
                demonstrations.getJSONObject(j).let { demo ->
                    demo.getString("label") to demo.getJSONArray("profiles").let { profiles ->
                        (0 until profiles.length()).map(profiles::getString).toSet()
                    }
                }
            })
        }
    }
    return UiCatalog(groups, profiles, articles)
}

fun JSONObject.toMap(): Map<String, Any> = keys().asSequence().associateWith(::get)
