package com.vss

import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import kotlinx.coroutines.delay

@Composable
fun VssTheme(content: @Composable () -> Unit) {
    val scheme = if (android.os.Build.VERSION.SDK_INT >= 31) {
        dynamicDarkColorScheme(LocalContext.current)
    } else {
        darkColorScheme(primary = Color(0xff55dfc1), surface = Color(0xff111516), background = Color(0xff090d0e))
    }
    MaterialTheme(colorScheme = scheme, content = content)
}

@Composable
fun SimulatorScreen(
    model: SimulatorViewModel,
    controller: SimulatorController,
    camera: () -> Unit,
    media: () -> Unit,
    settings: () -> Unit,
) {
    val activity = LocalContext.current as ComponentActivity
    val bars = remember(activity) { WindowCompat.getInsetsController(activity.window, activity.window.decorView) }
    var showFullscreenHint by remember { mutableStateOf(false) }
    LaunchedEffect(model.fullscreen) {
        if (model.fullscreen) bars.hide(WindowInsetsCompat.Type.systemBars()) else bars.show(WindowInsetsCompat.Type.systemBars())
    }
    BackHandler(model.fullscreen) { model.fullscreen = false }
    if (model.fullscreen) {
        LaunchedEffect(Unit) {
            showFullscreenHint = true
            delay(2500)
            showFullscreenHint = false
        }
        Box(Modifier.fillMaxSize()) {
            SimulatorPreview(
                model, controller, camera, media, settings,
                Modifier.fillMaxSize().clickable { model.fullscreen = false },
                showControls = false,
            )
            if (showFullscreenHint) {
                Surface(
                    Modifier.align(Alignment.BottomCenter).padding(24.dp),
                    shape = MaterialTheme.shapes.large,
                    color = MaterialTheme.colorScheme.inverseSurface.copy(alpha = 0.9f),
                ) {
                    Text(
                        stringResource(R.string.exit_fullscreen_hint),
                        Modifier.padding(horizontal = 20.dp, vertical = 12.dp),
                        color = MaterialTheme.colorScheme.inverseOnSurface,
                    )
                }
            }
        }
        return
    }

    val wide = LocalConfiguration.current.screenWidthDp >= 760
    Surface(Modifier.fillMaxSize()) {
        if (wide) {
            Row(Modifier.padding(12.dp), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                SimulatorPreview(model, controller, camera, media, settings, Modifier.weight(1.7f).fillMaxHeight(), true)
                CatalogPanel(model, controller, Modifier.widthIn(min = 360.dp).weight(1f))
            }
        } else {
            Column {
                SimulatorPreview(model, controller, camera, media, settings, Modifier.fillMaxWidth().aspectRatio(16f / 9f), true)
                CatalogPanel(model, controller, Modifier.weight(1f))
            }
        }
    }
}
