package com.vss

import android.content.Context
import android.hardware.HardwareBuffer
import android.net.Uri
import android.util.Size
import android.view.Surface
import com.vss.simulator.SimulatorBridge
import java.nio.ByteBuffer
import java.util.concurrent.atomic.AtomicReference

class SimulatorController(private val context: Context, private val permissionDenied: () -> Unit) {
    private val rendererLock = Any()
    @Volatile private var ready = false
    private var surface: Surface? = null
    private var camera: CameraFrameSource? = null
    private var media: MediaFrameSource? = null
    private val settings = AtomicReference<EyeSettings?>()
    private val pendingSettings = AtomicReference<EyeSettings?>()
    private val pendingSize = AtomicReference<Size?>()
    private val eyeMode = AtomicReference(EyeMode.LEFT)
    private val pendingEyeMode = AtomicReference<EyeMode?>()

    fun attach(surface: Surface, width: Int, height: Int) = synchronized(rendererLock) {
        detachLocked()
        this.surface = surface
        SimulatorBridge.create(surface, context.assets)
        SimulatorBridge.resize(width, height)
        ready = true
        SimulatorBridge.setEyeMode(eyeMode.get().bridgeValue)
        settings.get()?.let { SimulatorBridge.postSettings(it.left, it.right) }
    }
    fun resize(width: Int, height: Int) {
        if (isReady()) pendingSize.set(Size(width, height))
    }
    fun draw() = synchronized(rendererLock) {
        if (isReady()) {
            pendingSize.getAndSet(null)?.let { SimulatorBridge.resize(it.width, it.height) }
            pendingSettings.getAndSet(null)?.let { SimulatorBridge.postSettings(it.left, it.right) }
            pendingEyeMode.getAndSet(null)?.let { SimulatorBridge.setEyeMode(it.bridgeValue) }
            SimulatorBridge.draw()
        }
    }
    fun detach() = synchronized(rendererLock) { detachLocked() }
    private fun detachLocked() { ready = false; pendingSize.set(null); pendingSettings.set(null); pendingEyeMode.set(null); if (surface != null) SimulatorBridge.destroy(); surface = null }
    fun isReady() = ready
    fun startCamera() {
        stopSources()
        camera = CameraFrameSource(context, object : CameraFrameSource.CameraDelegate {
            override fun onCameraPermissionDenied() = permissionDenied()
            override fun onFrameAvailable(width: Int, height: Int, dataSpace: Int, rotationDegrees: Int, hardwareBuffer: HardwareBuffer) = postHardwareBuffer(width, height, dataSpace, rotationDegrees, hardwareBuffer)
        })
    }
    fun startMedia(uri: Uri, mimeType: String?) { stopSources(); media = MediaFrameSource(context, this).also { it.start(uri, mimeType) } }
    fun postHardwareBuffer(width: Int, height: Int, dataSpace: Int, rotationDegrees: Int, buffer: HardwareBuffer) { if (isReady()) SimulatorBridge.postHardwareBuffer(width, height, dataSpace, rotationDegrees, buffer) }
    fun postRgba(width: Int, height: Int, pixels: ByteBuffer) { if (isReady()) SimulatorBridge.postRgba(width, height, pixels) }
    fun postSettings(left: String, right: String) {
        val value = EyeSettings(left, right)
        settings.set(value)
        if (isReady()) pendingSettings.set(value)
    }
    fun setEyeMode(mode: EyeMode) {
        eyeMode.set(mode)
        if (isReady()) pendingEyeMode.set(mode)
    }
    fun semanticInput(kind: String, x: Float = 0f, y: Float = 0f) {
        if (isReady()) SimulatorBridge.semanticInput(kind, x, y)
    }
    fun stopSources() { camera?.close(); camera = null; media?.close(); media = null }
    fun close() { stopSources(); detach() }

    private data class EyeSettings(val left: String, val right: String)
    private val EyeMode.bridgeValue get() = name.lowercase()
}
