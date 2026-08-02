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
    private val settings = AtomicReference<String?>()
    private val pendingSettings = AtomicReference<String?>()
    private val pendingSize = AtomicReference<Size?>()

    fun attach(surface: Surface, width: Int, height: Int) = synchronized(rendererLock) {
        detachLocked()
        this.surface = surface
        SimulatorBridge.create(surface, context.assets)
        SimulatorBridge.resize(width, height)
        ready = true
        settings.get()?.let(SimulatorBridge::postSettings)
    }
    fun resize(width: Int, height: Int) {
        if (isReady()) pendingSize.set(Size(width, height))
    }
    fun draw() = synchronized(rendererLock) {
        if (isReady()) {
            pendingSize.getAndSet(null)?.let { SimulatorBridge.resize(it.width, it.height) }
            pendingSettings.getAndSet(null)?.let(SimulatorBridge::postSettings)
            SimulatorBridge.draw()
        }
    }
    fun detach() = synchronized(rendererLock) { detachLocked() }
    private fun detachLocked() { ready = false; pendingSize.set(null); pendingSettings.set(null); if (surface != null) SimulatorBridge.destroy(); surface = null }
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
    fun postSettings(json: String) {
        settings.set(json)
        if (isReady()) pendingSettings.set(json)
    }
    fun stopSources() { camera?.close(); camera = null; media?.close(); media = null }
    fun close() { stopSources(); detach() }
}
