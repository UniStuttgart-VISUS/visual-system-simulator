package com.vss

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Bundle
import android.provider.Settings
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.core.content.ContextCompat

class MainActivity : ComponentActivity() {
    private val model by viewModels<SimulatorViewModel>()
    private lateinit var controller: SimulatorController
    private val cameraPermission = registerForActivityResult(ActivityResultContracts.RequestPermission()) { granted ->
        model.cameraDenied = !granted
        if (granted) controller.startCamera()
    }
    private val mediaPicker = registerForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        uri?.let { startMedia(it, contentResolver.getType(it)) }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        controller = SimulatorController(this, ::requestCamera)
        setContent {
            VssTheme {
                SimulatorScreen(
                    model,
                    controller,
                    ::requestCamera,
                    { mediaPicker.launch(arrayOf("image/*", "video/*")) },
                    ::openAppSettings,
                )
            }
        }
        handleSendMediaIntent(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleSendMediaIntent(intent)
    }

    override fun onDestroy() {
        controller.close()
        super.onDestroy()
    }

    private fun requestCamera() {
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
            model.cameraDenied = false
            controller.startCamera()
        } else {
            cameraPermission.launch(Manifest.permission.CAMERA)
        }
    }

    private fun startMedia(uri: Uri, mime: String?) {
        model.sourceUri = uri
        model.sourceMime = mime
        model.cameraDenied = false
        controller.startMedia(uri, mime)
    }

    private fun handleSendMediaIntent(intent: Intent?) {
        if (intent?.action != Intent.ACTION_SEND) return
        @Suppress("DEPRECATION")
        val uri = intent.getParcelableExtra<Uri>(Intent.EXTRA_STREAM) ?: return
        startMedia(uri, intent.type)
    }

    private fun openAppSettings() = startActivity(
        Intent(Settings.ACTION_APPLICATION_DETAILS_SETTINGS, Uri.parse("package:$packageName")),
    )
}
