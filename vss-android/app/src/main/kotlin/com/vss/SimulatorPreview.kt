package com.vss

import androidx.compose.foundation.AndroidExternalSurface
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.res.stringResource
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
        AndroidExternalSurface(Modifier.fillMaxSize()) {
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
        }
    }
}
