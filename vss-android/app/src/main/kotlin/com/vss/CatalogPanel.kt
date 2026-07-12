package com.vss

import android.graphics.BitmapFactory
import android.graphics.drawable.BitmapDrawable
import android.graphics.drawable.ColorDrawable
import android.text.method.LinkMovementMethod
import android.widget.TextView
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Article
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.text.HtmlCompat
import com.vss.simulator.SimulatorBridge
import kotlinx.coroutines.delay
import org.json.JSONArray
import org.json.JSONObject
import java.util.Locale

@Composable
fun CatalogPanel(model: SimulatorViewModel, controller: SimulatorController, modifier: Modifier) {
    Column(modifier) {
        Articles(model)
        Profiles(model)
        SettingsPane(model, controller, Modifier.weight(1f))
    }
}

@Composable
private fun Articles(model: SimulatorViewModel) {
    val catalog = rememberCatalog()
    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 8.dp, vertical = 4.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        catalog.articles.forEach { article ->
            AssistChip(
                onClick = { model.articleId = article.id },
                label = { Text(article.title) },
                leadingIcon = { Icon(Icons.AutoMirrored.Filled.Article, null) },
            )
        }
    }
}

@Composable
private fun Profiles(model: SimulatorViewModel) {
    val catalog = rememberCatalog()
    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(8.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
        catalog.profiles.forEach { profile ->
            FilterChip(
                profile.id in model.activeProfiles,
                {
                    if (profile.id !in model.activeProfiles) model.activeProfiles += profile.id
                    else if (profile.values.keys.any { it in model.overrides }) model.pendingProfileRemoval = profile.id
                    else model.activeProfiles -= profile.id
                },
                { Text(profile.label) },
            )
        }
        if (model.overrides.isNotEmpty()) {
            AssistChip({ model.overrides = emptyMap() }, { Text(stringResource(R.string.reset_manual_changes)) }, leadingIcon = { Icon(Icons.Default.RestartAlt, null) })
        }
    }
    model.pendingProfileRemoval?.let { id ->
        catalog.profiles.firstOrNull { it.id == id }?.let { profile ->
            AlertDialog(
                onDismissRequest = { model.pendingProfileRemoval = null },
                title = { Text(stringResource(R.string.remove_profile, profile.label)) },
                text = { Text(stringResource(R.string.profile_affects_manual_changes)) },
                confirmButton = { TextButton({ model.activeProfiles -= id; model.pendingProfileRemoval = null }) { Text(stringResource(R.string.keep_overrides)) } },
                dismissButton = { Row {
                    TextButton({ model.activeProfiles -= id; model.overrides -= profile.values.keys; model.pendingProfileRemoval = null }) { Text(stringResource(R.string.discard_affected_overrides)) }
                    TextButton({ model.pendingProfileRemoval = null }) { Text(stringResource(R.string.cancel)) }
                } },
            )
        }
    }
}

@Composable
private fun SettingsPane(model: SimulatorViewModel, controller: SimulatorController, modifier: Modifier) {
    val catalog = rememberCatalog()
    val effectiveJson = remember(model.activeProfiles, model.overrides) {
        SimulatorBridge.composeSettings(Locale.getDefault().toLanguageTag(), JSONArray(model.activeProfiles.toList()).toString(), JSONObject(model.overrides).toString())
    }
    val effective = remember(effectiveJson) { JSONObject(effectiveJson).toMap() }
    LaunchedEffect(effectiveJson) { delay(100); controller.postSettings(effectiveJson) }
    Column(modifier.padding(horizontal = 8.dp).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
        catalog.groups.forEach { GroupCard(it, model, effective) }
        Spacer(Modifier.height(16.dp))
    }
    model.articleId?.let { id -> catalog.articles.firstOrNull { it.id == id }?.let { ArticleSheet(it, model) } }
    model.pendingDemoProfiles?.let { profiles ->
        AlertDialog(
            onDismissRequest = { model.pendingDemoProfiles = null },
            title = { Text(stringResource(R.string.apply_demonstration)) },
            text = { Text(stringResource(R.string.apply_demonstration_message)) },
            confirmButton = { Button({ model.activeProfiles = profiles; model.overrides = emptyMap(); model.pendingDemoProfiles = null }) { Text(stringResource(R.string.replace_current_simulation)) } },
            dismissButton = { Row {
                TextButton({ model.activeProfiles += profiles; model.pendingDemoProfiles = null }) { Text(stringResource(R.string.add_profiles)) }
                TextButton({ model.pendingDemoProfiles = null }) { Text(stringResource(R.string.cancel)) }
            } },
        )
    }
}

@Composable
private fun GroupCard(group: UiGroup, model: SimulatorViewModel, effective: Map<String, Any>) {
    val open = model.expanded[group.id] ?: true
    val enabled = group.parameters.firstOrNull { it.kind == "boolean" }
    Card { Column {
        Row(Modifier.fillMaxWidth().clickable { model.expanded[group.id] = !open }.padding(start = 16.dp, end = 8.dp, top = 8.dp, bottom = 8.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(group.title, style = MaterialTheme.typography.titleMedium)
            Spacer(Modifier.weight(1f))
            enabled?.let { parameter -> Switch(effective[parameter.key] as? Boolean ?: false, { model.overrides += parameter.key to it }) }
            Icon(if (open) Icons.Default.ExpandLess else Icons.Default.ExpandMore, null)
        }
        if (open) group.parameters.filterNot { it === enabled }.forEach { SettingRow(it, model, effective) }
    } }
}

@Composable
private fun SettingRow(parameter: UiParameter, model: SimulatorViewModel, effective: Map<String, Any>) {
    val value = effective[parameter.key] ?: parameter.default
    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalAlignment = Alignment.CenterVertically) {
        Row(Modifier.weight(1f), verticalAlignment = Alignment.CenterVertically) {
            Text(parameter.label)
            if (parameter.key in model.overrides) IconButton({ model.overrides -= parameter.key }, Modifier.size(36.dp)) { Icon(Icons.Default.RestartAlt, stringResource(R.string.reset)) }
        }
        when (parameter.kind) {
            "boolean" -> Switch(value as? Boolean ?: false, { model.overrides += parameter.key to it })
            "number" -> NumericControl(value as Number, parameter) { model.overrides += parameter.key to it }
            "choice" -> Row { parameter.choices.forEach { choice -> FilterChip(choice.value == (value as Number).toInt(), { model.overrides += parameter.key to choice.value }, { Text(choice.label) }) } }
            else -> Text("$value")
        }
    }
}

@Composable
private fun NumericControl(value: Number, parameter: UiParameter, change: (Double) -> Unit) {
    val focusManager = LocalFocusManager.current
    var text by remember(value) { mutableStateOf(value.toString()) }
    fun commit() { text.toDoubleOrNull()?.let { change(it.coerceIn(parameter.min ?: -Double.MAX_VALUE, parameter.max ?: Double.MAX_VALUE)) } }
    Row(verticalAlignment = Alignment.CenterVertically) {
        IconButton({ change((value.toDouble() - parameter.step).coerceAtLeast(parameter.min ?: -Double.MAX_VALUE)) }) { Icon(Icons.Default.Remove, null) }
        OutlinedTextField(text, { text = it }, Modifier.width(104.dp), singleLine = true, suffix = { parameter.unit?.let { Text(it) } }, keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Decimal, imeAction = ImeAction.Done), keyboardActions = KeyboardActions(onDone = { commit(); focusManager.clearFocus() }))
        IconButton({ change((value.toDouble() + parameter.step).coerceAtMost(parameter.max ?: Double.MAX_VALUE)) }) { Icon(Icons.Default.Add, null) }
    }
}

@Composable
private fun rememberCatalog() = remember { parseCatalog(SimulatorBridge.catalog(Locale.getDefault().toLanguageTag())) }

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ArticleSheet(article: UiArticle, model: SimulatorViewModel) {
    ModalBottomSheet(onDismissRequest = { model.articleId = null }) {
        Column(Modifier.fillMaxWidth().padding(24.dp).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(article.title, style = MaterialTheme.typography.headlineSmall)
            HtmlArticle(article.contentPath)
            article.demonstrations.forEach { (label, profiles) -> Button({ model.pendingDemoProfiles = profiles; model.articleId = null }) { Text(label) } }
            Spacer(Modifier.height(24.dp))
        }
    }
}

@Composable
private fun HtmlArticle(contentPath: String) {
    val context = LocalContext.current
    val html = remember(contentPath) { context.assets.open(contentPath).bufferedReader().use { it.readText() } }
    val textColor = MaterialTheme.colorScheme.onSurface.toArgb()
    val linkColor = MaterialTheme.colorScheme.primary.toArgb()
    AndroidView(
        factory = { TextView(context).apply { setTextColor(textColor); setLinkTextColor(linkColor); textSize = 16f; movementMethod = LinkMovementMethod.getInstance() } },
        update = { view ->
            val imageGetter = android.text.Html.ImageGetter { source ->
                runCatching {
                    context.assets.open(source).use { input ->
                        val bitmap = BitmapFactory.decodeStream(input)
                        val scale = minOf(1f, (context.resources.displayMetrics.widthPixels - 96).toFloat() / bitmap.width)
                        BitmapDrawable(context.resources, bitmap).apply { setBounds(0, 0, (bitmap.width * scale).toInt(), (bitmap.height * scale).toInt()) }
                    }
                }.getOrElse { ColorDrawable(android.graphics.Color.TRANSPARENT).apply { setBounds(0, 0, 1, 1) } }
            }
            view.text = HtmlCompat.fromHtml(html, HtmlCompat.FROM_HTML_MODE_COMPACT, imageGetter, null)
        },
    )
}
