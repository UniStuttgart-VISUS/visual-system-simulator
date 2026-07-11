package com.vss

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.hardware.HardwareBuffer
import android.net.Uri
import android.os.Bundle
import android.os.StrictMode
import android.util.Log
import android.view.View
import android.view.WindowManager
import android.webkit.JavascriptInterface
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.appcompat.app.AppCompatActivity
import androidx.core.app.ActivityCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import androidx.slidingpanelayout.widget.SlidingPaneLayout
import com.google.android.material.floatingactionbutton.FloatingActionButton
import com.vss.simulator.SimulatorSurfaceView
import java.util.concurrent.FutureTask

/** Main activity. */
class MainActivity : AppCompatActivity(), ActivityCompat.OnRequestPermissionsResultCallback {
    private var activityState = ActivityState.Welcome
    private lateinit var inspectorSimulatorPane: SlidingPaneLayout
    private lateinit var startButton: FloatingActionButton
    private lateinit var galleryButton: FloatingActionButton
    private lateinit var inspectorView: WebView
    private lateinit var simulatorView: SimulatorSurfaceView
    private var cameraFrameSource: CameraFrameSource? = null
    private var mediaFrameSource: MediaFrameSource? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        if (BuildConfig.DEBUG) {
            Log.w(LOG_TAG, "======= APPLICATION IN STRICT MODE - DEBUGGING =======")
            StrictMode.setVmPolicy(
                StrictMode.VmPolicy.Builder()
                    .detectAll()
                    .penaltyLog()
                    .build(),
            )
            StrictMode.setThreadPolicy(
                StrictMode.ThreadPolicy.Builder()
                    .detectAll()
                    .permitDiskReads()
                    .penaltyFlashScreen()
                    .penaltyLog()
                    .build(),
            )
        }

        setupInspectorSimulatorChanger()
        setupInspectorView()
        simulatorView = findViewById(R.id.simulator_view)
        handleSendMediaIntent(intent)
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        setIntent(intent)
        handleSendMediaIntent(intent)
    }

    override fun onDestroy() {
        stopCamera()
        stopMedia()
        super.onDestroy()
    }

    @Deprecated("Deprecated in Android")
    override fun onBackPressed() {
        when (activityState) {
            ActivityState.Simulating -> stopSimulator()
            ActivityState.Inspecting -> {
                inspectorView.loadUrl("file:///android_asset/index.html")
                activityState = ActivityState.Welcome
            }
            ActivityState.Welcome -> super.onBackPressed()
        }
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<out String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        if (requestCode == CAMERA_REQUEST_CODE) {
            if (grantResults[0] == PackageManager.PERMISSION_GRANTED) {
                Log.w("Permission", "Camera: GRANTED")
            } else {
                Log.i("Permission", "Camera: DENIED")
            }
        }
    }

    private fun checkCameraPermission() {
        if (
            ActivityCompat.checkSelfPermission(this, Manifest.permission.CAMERA) ==
            PackageManager.PERMISSION_DENIED
        ) {
            ActivityCompat.requestPermissions(
                this,
                arrayOf(Manifest.permission.CAMERA),
                CAMERA_REQUEST_CODE,
            )
        }
    }

    private fun setupInspectorSimulatorChanger() {
        inspectorSimulatorPane = findViewById(R.id.inspector_simulator_pane)
        inspectorSimulatorPane.setLockMode(SlidingPaneLayout.LOCK_MODE_LOCKED)
        startButton = findViewById(R.id.start_button)
        galleryButton = findViewById(R.id.gallery_button)
    }

    fun startStopClicked(view: View) {
        startSimulator()
    }

    fun openMediaClicked(view: View) {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = "*/*"
            putExtra(Intent.EXTRA_MIME_TYPES, arrayOf("image/*", "video/*"))
            addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION)
        }
        startActivityForResult(intent, MEDIA_REQUEST_CODE)
    }

    private fun setupInspectorView() {
        inspectorView = findViewById(R.id.inspector_view)
        inspectorView.isHorizontalScrollBarEnabled = false
        inspectorView.webViewClient = object : WebViewClient() {
            override fun shouldOverrideUrlLoading(
                view: WebView,
                request: WebResourceRequest,
            ): Boolean {
                val url = request.url
                activityState = if (url.path?.endsWith("index.html") == true) {
                    ActivityState.Welcome
                } else {
                    ActivityState.Inspecting
                }
                Log.i(LOG_TAG, "Loading URL: $url")
                return url.scheme != "file"
            }
        }
        inspectorView.settings.javaScriptEnabled = true
        inspectorView.addJavascriptInterface(this, "Activity")
        inspectorView.settings.saveFormData = false
        inspectorView.loadUrl("file:///android_asset/index.html")
        activityState = ActivityState.Welcome
    }

    @JavascriptInterface
    fun querySettings(): String {
        val futureResult = FutureTask { simulatorView.querySettings() }
        runOnUiThread(futureResult)
        return futureResult.get()
    }

    @JavascriptInterface
    fun postSettings(jsonString: String) {
        runOnUiThread { simulatorView.postSettings(jsonString) }
    }

    fun startSimulator() {
        Log.d(LOG_TAG, "Starting simulator")
        stopMedia()
        startCamera()
        enterSimulationState()
    }

    private fun startMedia(uri: Uri, mimeType: String?) {
        Log.d(LOG_TAG, "Starting simulator from media: $uri")
        stopCamera()
        enterSimulationState()
        val source = mediaFrameSource ?: MediaFrameSource(this, simulatorView).also {
            mediaFrameSource = it
        }
        source.start(uri, mimeType)
    }

    private fun enterSimulationState() {
        val windowInsetsController = WindowCompat.getInsetsController(window, window.decorView)
        windowInsetsController.systemBarsBehavior =
            WindowInsetsControllerCompat.BEHAVIOR_SHOW_BARS_BY_TOUCH
        windowInsetsController.hide(WindowInsetsCompat.Type.systemBars())
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)

        inspectorSimulatorPane.open()
        startButton.hide()
        galleryButton.hide()
        activityState = ActivityState.Simulating
    }

    private fun startCamera() {
        cameraFrameSource = CameraFrameSource(this, object : CameraFrameSource.CameraDelegate {
            override fun onCameraPermissionDenied() {
                checkCameraPermission()
            }

            override fun onFrameAvailable(
                width: Int,
                height: Int,
                dataSpace: Int,
                rotationDegrees: Int,
                hardwareBuffer: HardwareBuffer,
            ) {
                simulatorView.postHardwareBuffer(
                    width,
                    height,
                    dataSpace,
                    rotationDegrees,
                    hardwareBuffer,
                )
            }
        })
    }

    internal fun stopSimulator() {
        stopCamera()
        stopMedia()

        val windowInsetsController = WindowCompat.getInsetsController(window, window.decorView)
        windowInsetsController.show(WindowInsetsCompat.Type.systemBars())
        window.clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)

        inspectorSimulatorPane.close()
        startButton.show()
        galleryButton.show()
        activityState = ActivityState.Inspecting
    }

    @Deprecated("Deprecated in Android")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != MEDIA_REQUEST_CODE || resultCode != RESULT_OK || data == null) {
            return
        }

        val uri = data.data
        if (uri == null) {
            Log.w(LOG_TAG, "Media picker did not return a URI")
            return
        }

        takeReadPermission(uri, data.flags)
        startMedia(uri, data.type)
    }

    private fun stopCamera() {
        cameraFrameSource?.close()
        cameraFrameSource = null
    }

    private fun stopMedia() {
        mediaFrameSource?.close()
        mediaFrameSource = null
    }

    @Suppress("DEPRECATION")
    private fun handleSendMediaIntent(intent: Intent?) {
        if (intent?.action != Intent.ACTION_SEND) {
            return
        }

        val uri = intent.getParcelableExtra<Uri>(Intent.EXTRA_STREAM)
        if (uri == null) {
            Log.w(LOG_TAG, "Shared media intent did not contain a stream")
            return
        }

        takeReadPermission(uri, intent.flags)
        startMedia(uri, intent.type)
    }

    private fun takeReadPermission(uri: Uri, flags: Int) {
        try {
            contentResolver.takePersistableUriPermission(
                uri,
                flags and Intent.FLAG_GRANT_READ_URI_PERMISSION,
            )
        } catch (error: SecurityException) {
            Log.d(LOG_TAG, "Media URI is not persistable; using transient grant")
        }
    }

    private enum class ActivityState {
        Welcome,
        Simulating,
        Inspecting,
    }

    private companion object {
        const val LOG_TAG = "MainActivity"
        const val CAMERA_REQUEST_CODE = 100
        const val MEDIA_REQUEST_CODE = 101
    }
}
