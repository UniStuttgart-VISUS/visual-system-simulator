package com.vss

import android.content.pm.ActivityInfo
import androidx.activity.ComponentActivity
import androidx.activity.compose.BackHandler
import androidx.compose.foundation.layout.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.unit.dp
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat

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
    LaunchedEffect(model.fullscreen) {
        if (model.fullscreen) bars.hide(WindowInsetsCompat.Type.systemBars()) else bars.show(WindowInsetsCompat.Type.systemBars())
    }
    val requestLandscape = model.fullscreen && model.simulator.eyeMode == EyeMode.BOTH
    DisposableEffect(activity, requestLandscape) {
        if (!requestLandscape) return@DisposableEffect onDispose { }
        val previous = activity.requestedOrientation
        activity.requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
        onDispose { activity.requestedOrientation = previous }
    }
    BackHandler(model.fullscreen) { model.fullscreen = false }
    Surface(Modifier.fillMaxSize()) {
        BoxWithConstraints {
            val wide = maxWidth >= 760.dp
            val previewHeight = maxWidth / (16f / 9f)
            val contentWidth = maxWidth - 36.dp
            val widePreviewWidth = contentWidth * (1.7f / 2.7f)
            val previewModifier = when {
                model.fullscreen -> Modifier.fillMaxSize()
                wide -> Modifier.offset(12.dp, 12.dp).width(widePreviewWidth).height(maxHeight - 24.dp)
                else -> Modifier.fillMaxWidth().height(previewHeight)
            }
            val catalogModifier = when {
                model.fullscreen -> Modifier.offset(y = maxHeight).fillMaxSize()
                wide -> Modifier.offset(widePreviewWidth + 24.dp, 12.dp)
                    .width(contentWidth - widePreviewWidth).height(maxHeight - 24.dp)
                else -> Modifier.offset(y = previewHeight).fillMaxWidth().height(maxHeight - previewHeight)
            }

            SimulatorPreview(model, controller, camera, media, settings, previewModifier, !model.fullscreen)
            CatalogPanel(model, controller, catalogModifier)
        }
    }
}
