package com.vss

import android.graphics.BitmapFactory
import android.graphics.drawable.BitmapDrawable
import android.graphics.drawable.ColorDrawable
import android.text.method.LinkMovementMethod
import android.widget.TextView
import androidx.compose.foundation.Image
import androidx.compose.foundation.ExperimentalFoundationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Article
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.asImageBitmap
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.toArgb
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.text.HtmlCompat
import com.vss.simulator.SimulatorBridge
import kotlinx.coroutines.flow.distinctUntilChanged
import org.json.JSONObject
import java.util.Locale

@OptIn(ExperimentalFoundationApi::class)
@Composable
fun CatalogPanel(model: SimulatorViewModel, controller: SimulatorController, modifier: Modifier) {
    val catalog = rememberCatalog()
    val activePresets = model.simulator.activePresets(catalog)
    val initialArticle = remember {
        model.viewedArticleId?.let { id -> catalog.articles.indexOfFirst { it.id == id } }?.takeIf { it >= 0 }
            ?: catalog.articles.indexOfFirst { article -> article.demonstrations.any { demo -> demo.presets.any(activePresets::contains) } }.takeIf { it >= 0 }
            ?: 0
    }
    val galleryState = rememberLazyListState(initialFirstVisibleItemIndex = initialArticle)
    val verticalState = rememberScrollState()
    val effective = model.simulator.editableValues(catalog)
    val leftJson = remember(model.simulator) { JSONObject(model.simulator.effectiveValues(catalog, EyeMode.LEFT)).toString() }
    val rightJson = remember(model.simulator) { JSONObject(model.simulator.effectiveValues(catalog, EyeMode.RIGHT)).toString() }
    LaunchedEffect(leftJson, rightJson) { controller.postSettings(leftJson, rightJson) }
    LaunchedEffect(model.simulator.eyeMode) { controller.setEyeMode(model.simulator.eyeMode) }
    LaunchedEffect(galleryState) {
        snapshotFlow {
            val layout = galleryState.layoutInfo
            layout.visibleItemsInfo.minByOrNull { item ->
                kotlin.math.abs(item.offset - layout.viewportStartOffset)
            }?.index
        }.distinctUntilChanged().collect { index ->
            index?.let(catalog.articles::getOrNull)?.let { model.viewedArticleId = it.id }
        }
    }

    Column(modifier.verticalScroll(verticalState).padding(vertical = 8.dp), verticalArrangement = Arrangement.spacedBy(10.dp)) {
        ArticleIndex(catalog, model.viewedArticleId ?: catalog.articles.getOrNull(initialArticle)?.id, activePresets)
        LazyRow(
            state = galleryState,
            flingBehavior = androidx.compose.foundation.gestures.snapping.rememberSnapFlingBehavior(galleryState),
            contentPadding = PaddingValues(horizontal = 12.dp),
            horizontalArrangement = Arrangement.spacedBy(10.dp),
            modifier = Modifier.fillMaxWidth(),
        ) {
            itemsIndexed(catalog.articles, key = { _, article -> article.id }) { index, article ->
                ArticleCard(article, index, catalog, model, Modifier.fillParentMaxWidth(.88f))
            }
        }
        catalog.groups.forEach { GroupCard(it, catalog, model, effective) }
        Spacer(Modifier.height(16.dp))
    }
    model.articleId?.let { id -> catalog.articles.firstOrNull { it.id == id }?.let { ArticleSheet(it, catalog, model) } }
}

@Composable
private fun ArticleCard(article: UiArticle, index: Int, catalog: UiCatalog, model: SimulatorViewModel, modifier: Modifier) {
    val selected = article.demonstrations.firstOrNull { it.id == model.simulator.selectedDemonstration(article.id) }
    val description = buildString {
        append("${article.title}, ${index + 1} of ${catalog.articles.size}")
        selected?.let { append(", ${it.label} active") }
        append(", open article")
    }
    Card(modifier.height(260.dp).semantics(mergeDescendants = true) { contentDescription = description }) {
        Column(Modifier.fillMaxSize()) {
            Box(Modifier.fillMaxWidth().weight(1f).clickable { model.articleId = article.id }) {
                GalleryImage(article, Modifier.fillMaxSize())
                Row(
                    Modifier.align(Alignment.BottomStart).fillMaxWidth()
                        .background(Color.Black.copy(alpha = .62f)).padding(start = 16.dp, end = 6.dp, top = 6.dp, bottom = 6.dp),
                    verticalAlignment = Alignment.CenterVertically,
                ) {
                    Text(article.title, color = Color.White, style = MaterialTheme.typography.titleLarge, maxLines = 2, modifier = Modifier.weight(1f))
                    IconButton({ model.articleId = article.id }) {
                        Icon(Icons.Default.Info, stringResource(R.string.open_article), tint = Color.White)
                    }
                }
            }
            if (article.demonstrations.isNotEmpty()) DemonstrationSegments(article, catalog, model)
            else Spacer(Modifier.fillMaxWidth().height(64.dp))
        }
    }
}

@Composable
private fun GalleryImage(article: UiArticle, modifier: Modifier = Modifier) {
    val context = LocalContext.current
    val bitmap = remember(article.image) {
        article.image?.let { path -> runCatching { context.assets.open(path).use(BitmapFactory::decodeStream) }.getOrNull() }
    }
    if (bitmap != null) {
        Image(bitmap.asImageBitmap(), null, modifier, contentScale = ContentScale.Crop)
    } else {
        Box(modifier.background(MaterialTheme.colorScheme.surfaceVariant), contentAlignment = Alignment.Center) {
            Icon(Icons.AutoMirrored.Filled.Article, null, Modifier.size(52.dp), tint = MaterialTheme.colorScheme.onSurfaceVariant)
        }
    }
}

@Composable
private fun DemonstrationSegments(article: UiArticle, catalog: UiCatalog, model: SimulatorViewModel) {
    Row(Modifier.fillMaxWidth().height(64.dp).padding(horizontal = 8.dp, vertical = 6.dp), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        article.demonstrations.forEach { demo ->
            val selected = model.simulator.selectedDemonstration(article.id) == demo.id
            val label = if (demo.label.equals(article.title, ignoreCase = true)) {
                stringResource(if (selected) R.string.deactivate else R.string.activate)
            } else demo.label
            FilterChip(
                selected = selected,
                onClick = { model.simulator = model.simulator.selectDemonstration(catalog, article.id, demo.id) },
                label = { Text(label) },
                modifier = Modifier.weight(1f),
            )
        }
    }
}

@Composable
private fun ArticleIndex(catalog: UiCatalog, currentId: String?, activePresets: Set<String>) {
    Row(Modifier.fillMaxWidth().padding(horizontal = 12.dp).semantics {
        contentDescription = "Article position; active simulations are shown with thick segments"
    }, horizontalArrangement = Arrangement.spacedBy(3.dp), verticalAlignment = Alignment.CenterVertically) {
        catalog.articles.forEach { article ->
            val current = article.id == currentId
            val active = article.demonstrations.any { demo -> demo.presets.any(activePresets::contains) }
            Box(Modifier.weight(1f).height(if (current) 8.dp else if (active) 6.dp else 3.dp)
                .clip(RoundedCornerShape(50)).background(if (active) MaterialTheme.colorScheme.primary else if (current) MaterialTheme.colorScheme.secondary else MaterialTheme.colorScheme.outlineVariant))
        }
    }
}

@Composable
private fun GroupCard(group: UiGroup, catalog: UiCatalog, model: SimulatorViewModel, effective: Map<String, Any>) {
    val open = model.expanded[group.id] ?: true
    val enabled = group.settings.firstOrNull { it.kind == "boolean" }
    Card(Modifier.padding(horizontal = 8.dp)) { Column {
        Row(Modifier.fillMaxWidth().clickable { model.expanded[group.id] = !open }.padding(12.dp), verticalAlignment = Alignment.CenterVertically) {
            Text(group.title, style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            enabled?.let { setting -> Switch(effective[setting.id] as? Boolean ?: false, { model.simulator = model.simulator.edit(setting.id, it) }) }
            Icon(if (open) Icons.Default.ExpandLess else Icons.Default.ExpandMore, null)
        }
        if (open) group.settings.filterNot { it === enabled }.forEach { SettingRow(it, catalog, model, effective) }
    } }
}

@Composable
private fun SettingRow(setting: UiSetting, catalog: UiCatalog, model: SimulatorViewModel, effective: Map<String, Any>) {
    val value = effective[setting.id] ?: setting.default
    val sourceArticle = model.simulator.sourceArticle(catalog, setting.id)
    Row(Modifier.fillMaxWidth().padding(horizontal = 16.dp, vertical = 8.dp), verticalAlignment = Alignment.CenterVertically) {
        Text(setting.label, Modifier.weight(1f))
        when {
            setting.id in model.simulator.currentLayer().manual -> IconButton({ model.simulator = model.simulator.reset(setting.id, catalog) }, Modifier.size(40.dp)) { Icon(Icons.Default.RestartAlt, stringResource(R.string.reset)) }
            sourceArticle != null -> IconButton({ model.articleId = sourceArticle }, Modifier.size(40.dp)) { Icon(Icons.Default.Info, "Open explaining article") }
        }
        when (setting.kind) {
            "boolean" -> Switch(value as? Boolean ?: false, { model.simulator = model.simulator.edit(setting.id, it) })
            "number" -> NumericControl(value as Number, setting) { model.simulator = model.simulator.edit(setting.id, it) }
            "choice" -> Row { setting.choices.forEach { choice -> FilterChip(choice.value == (value as Number).toInt(), { model.simulator = model.simulator.edit(setting.id, choice.value) }, { Text(choice.label) }) } }
            else -> Text("$value")
        }
    }
}

@Composable
private fun NumericControl(value: Number, setting: UiSetting, change: (Double) -> Unit) {
    val focusManager = LocalFocusManager.current
    var text by remember(value) { mutableStateOf(value.toString()) }
    fun commit() { text.toDoubleOrNull()?.let { change(it.coerceIn(setting.min ?: -Double.MAX_VALUE, setting.max ?: Double.MAX_VALUE)) } }
    Row(verticalAlignment = Alignment.CenterVertically) {
        IconButton({ change((value.toDouble() - setting.step).coerceAtLeast(setting.min ?: -Double.MAX_VALUE)) }) { Icon(Icons.Default.Remove, null) }
        OutlinedTextField(text, { text = it }, Modifier.width(104.dp), singleLine = true, suffix = { setting.unit?.let { Text(it) } }, keyboardOptions = androidx.compose.foundation.text.KeyboardOptions(keyboardType = KeyboardType.Decimal, imeAction = ImeAction.Done), keyboardActions = androidx.compose.foundation.text.KeyboardActions(onDone = { commit(); focusManager.clearFocus() }))
        IconButton({ change((value.toDouble() + setting.step).coerceAtMost(setting.max ?: Double.MAX_VALUE)) }) { Icon(Icons.Default.Add, null) }
    }
}

@Composable
private fun rememberCatalog(): UiCatalog = remember {
    parseCatalog(SimulatorBridge.catalog(Locale.getDefault().toLanguageTag()))
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ArticleSheet(article: UiArticle, catalog: UiCatalog, model: SimulatorViewModel) {
    val sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)
    ModalBottomSheet(onDismissRequest = { model.articleId = null }, sheetState = sheetState) {
        Column(Modifier.fillMaxWidth().fillMaxHeight(.94f)) {
            Column(Modifier.weight(1f).padding(horizontal = 24.dp).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(12.dp)) {
                Text(article.title, style = MaterialTheme.typography.headlineSmall)
                HtmlArticle(article.contentPath)
            }
            if (article.demonstrations.isNotEmpty()) {
                HorizontalDivider()
                Row(Modifier.fillMaxWidth().padding(12.dp), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    article.demonstrations.forEach { demo -> FilterChip(
                        selected = model.simulator.selectedDemonstration(article.id) == demo.id,
                        onClick = { model.simulator = model.simulator.selectDemonstration(catalog, article.id, demo.id); model.articleId = null },
                        label = { Text(demo.label) }, modifier = Modifier.weight(1f),
                    ) }
                }
            }
        }
    }
}

@Composable
private fun HtmlArticle(contentPath: String) {
    val context = LocalContext.current
    val html = remember(contentPath) { context.assets.open(contentPath).bufferedReader().use { it.readText() } }
    val textColor = MaterialTheme.colorScheme.onSurface.toArgb()
    val linkColor = MaterialTheme.colorScheme.primary.toArgb()
    AndroidView(factory = { TextView(context).apply { setTextColor(textColor); setLinkTextColor(linkColor); textSize = 16f; movementMethod = LinkMovementMethod.getInstance() } }, update = { view ->
        val imageGetter = android.text.Html.ImageGetter { source -> runCatching { context.assets.open(source).use { input ->
            val bitmap = BitmapFactory.decodeStream(input); val scale = minOf(1f, (context.resources.displayMetrics.widthPixels - 96).toFloat() / bitmap.width)
            BitmapDrawable(context.resources, bitmap).apply { setBounds(0, 0, (bitmap.width * scale).toInt(), (bitmap.height * scale).toInt()) }
        } }.getOrElse { ColorDrawable(android.graphics.Color.TRANSPARENT).apply { setBounds(0, 0, 1, 1) } } }
        view.text = HtmlCompat.fromHtml(html, HtmlCompat.FROM_HTML_MODE_COMPACT, imageGetter, null)
    })
}
