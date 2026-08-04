package com.vss

import androidx.compose.foundation.AndroidEmbeddedExternalSurface
import androidx.compose.foundation.background
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.gestures.awaitEachGesture
import androidx.compose.foundation.gestures.awaitFirstDown
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.input.pointer.*
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.currentCoroutineContext
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import kotlinx.coroutines.withContext

@Composable
fun SimulatorPreview(
    model: SimulatorViewModel,
    controller: SimulatorController,
    camera: () -> Unit,
    media: () -> Unit,
    settings: () -> Unit,
    modifier: Modifier,
    showControls: Boolean,
) {
    var sourceMenu by remember { mutableStateOf(false) }
    Box(modifier.background(Color.Black)) {
        AndroidEmbeddedExternalSurface(Modifier.fillMaxSize().previewGestures(controller)) {
            onSurface { surface, width, height ->
                controller.attach(surface, width, height)
                model.sourceUri?.let { controller.startMedia(it, model.sourceMime) } ?: camera()
                surface.onChanged { width, height -> controller.resize(width, height) }
                surface.onDestroyed { controller.detach() }
            }
        }
        LaunchedEffect(controller) {
            withContext(Dispatchers.Default) {
                while (currentCoroutineContext().isActive) {
                    controller.draw()
                    delay(16)
                }
            }
        }
        if (model.cameraDenied) {
            Column(Modifier.align(Alignment.Center), horizontalAlignment = Alignment.CenterHorizontally) {
                Text(stringResource(R.string.camera_required))
                Row {
                    TextButton(camera) { Text(stringResource(R.string.allow_camera)) }
                    TextButton(settings) { Text(stringResource(R.string.app_settings)) }
                }
            }
        }
        if (showControls) {
            Box(Modifier.align(Alignment.BottomStart).padding(8.dp)) {
                FilledTonalIconButton({ sourceMenu = true }) {
                    Icon(Icons.Default.PhotoLibrary, stringResource(R.string.choose_source))
                }
                DropdownMenu(sourceMenu, { sourceMenu = false }) {
                    DropdownMenuItem(
                        { Text(stringResource(R.string.camera)) },
                        onClick = { sourceMenu = false; model.sourceUri = null; camera() },
                        leadingIcon = { Icon(Icons.Default.PhotoCamera, null) },
                    )
                    DropdownMenuItem(
                        { Text(stringResource(R.string.open_media)) },
                        onClick = { sourceMenu = false; media() },
                        leadingIcon = { Icon(Icons.Default.FolderOpen, null) },
                    )
                }
            }
            FilledTonalIconButton({ model.fullscreen = true }, Modifier.align(Alignment.BottomEnd).padding(8.dp)) {
                Icon(Icons.Default.Fullscreen, stringResource(R.string.enter_fullscreen))
            }
            EyeModeSelector(model, Modifier.align(Alignment.BottomCenter).padding(8.dp))
        }
    }
}

private fun Modifier.previewGestures(controller: SimulatorController) = pointerInput(controller) {
    var previousTapTime = 0L
    var previousTapPosition = androidx.compose.ui.geometry.Offset.Unspecified
    awaitEachGesture {
        val down = awaitFirstDown(requireUnconsumed = false)
        val start = down.position
        var moved = false
        var maximumPointers = 1
        var lastTime = down.uptimeMillis
        do {
            val event = awaitPointerEvent()
            val pressed = event.changes.filter { it.pressed }
            maximumPointers = maxOf(maximumPointers, pressed.size)
            val changed = pressed.filter { it.positionChanged() }
            if (changed.isNotEmpty()) {
                val delta = changed.map { it.position - it.previousPosition }
                    .reduce { total, next -> total + next } / changed.size.toFloat()
                if (delta.getDistance() > 0f) {
                    moved = moved || (changed.first().position - start).getDistance() > viewConfiguration.touchSlop
                    val kind = if (pressed.size >= 2) "view_delta" else "gaze_delta"
                    controller.semanticInput(kind, delta.x / size.width.coerceAtLeast(1), delta.y / size.height.coerceAtLeast(1))
                }
            }
            event.changes.forEach { if (it.positionChanged()) it.consume() }
            lastTime = event.changes.maxOfOrNull { it.uptimeMillis } ?: lastTime
        } while (event.changes.any { it.pressed })

        if (!moved && maximumPointers == 1) {
            val close = previousTapPosition != androidx.compose.ui.geometry.Offset.Unspecified && (start - previousTapPosition).getDistance() <= viewConfiguration.touchSlop * 2
            if (close && lastTime - previousTapTime <= viewConfiguration.doubleTapTimeoutMillis) {
                controller.semanticInput("reset_pose")
                previousTapTime = 0
                previousTapPosition = androidx.compose.ui.geometry.Offset.Unspecified
            } else {
                previousTapTime = lastTime
                previousTapPosition = start
            }
        }
    }
}

@Composable
private fun EyeModeSelector(model: SimulatorViewModel, modifier: Modifier = Modifier) {
    val options = listOf(
        EyeMode.LEFT to R.string.left_eye,
        EyeMode.BOTH to R.string.both_eyes,
        EyeMode.RIGHT to R.string.right_eye,
    )
    Row(modifier, horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        options.forEach { (mode, label) ->
            val description = stringResource(label)
            FilterChip(
                selected = model.simulator.eyeMode == mode,
                onClick = { model.simulator = model.simulator.withEyeMode(mode) },
                label = { EyeModeArtwork(mode) },
                modifier = Modifier.semantics { contentDescription = description },
            )
        }
    }
}

@Composable
private fun EyeModeArtwork(mode: EyeMode) {
    val color = androidx.compose.material3.LocalContentColor.current
    Canvas(Modifier.size(32.dp, 22.dp)) {
        val sx = size.width / 32f
        val sy = size.height / 22f
        val viewer = Path().apply {
            moveTo(3f * sx, 8.5f * sy); lineTo(6f * sx, 5f * sy); lineTo(26f * sx, 5f * sy)
            lineTo(29f * sx, 8.5f * sy); lineTo(29f * sx, 16.5f * sy); lineTo(26f * sx, 19f * sy)
            lineTo(6f * sx, 19f * sy); lineTo(3f * sx, 16.5f * sy); close()
        }
        val stroke = Stroke(width = 1.8f * minOf(sx, sy), cap = StrokeCap.Round)
        drawPath(viewer, color.copy(alpha = .72f), style = stroke)
        listOf(11f, 21f).forEachIndexed { index, x ->
            val active = mode == EyeMode.BOTH || (mode == EyeMode.LEFT && index == 0) || (mode == EyeMode.RIGHT && index == 1)
            drawCircle(color.copy(alpha = if (active) 1f else .35f), radius = 4f * minOf(sx, sy), center = androidx.compose.ui.geometry.Offset(x * sx, 12f * sy), style = if (active) androidx.compose.ui.graphics.drawscope.Fill else stroke)
        }
    }
}
