package com.vss.simulator

import android.content.Context
import android.hardware.HardwareBuffer
import android.os.Looper
import android.util.AttributeSet
import android.view.SurfaceHolder
import android.view.SurfaceView
import java.nio.ByteBuffer

/** Surface view for simulator rendering. */
class SimulatorSurfaceView : SurfaceView, SurfaceHolder.Callback2 {
    constructor(context: Context) : super(context) {
        initialize()
    }

    constructor(context: Context, attrs: AttributeSet?) : super(context, attrs) {
        initialize()
    }

    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) :
        super(context, attrs, defStyleAttr) {
        initialize()
    }

    private fun initialize() {
        alpha = 1.0f
        holder.addCallback(this)
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        assertMainThread()
        SimulatorBridge.create(holder.surface, resources.assets)
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        assertMainThread()
        SimulatorBridge.resize(width, height)
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        assertMainThread()
        SimulatorBridge.destroy()
    }

    override fun surfaceRedrawNeeded(holder: SurfaceHolder) {
        assertMainThread()
        SimulatorBridge.draw()
    }

    fun postHardwareBuffer(
        width: Int,
        height: Int,
        dataSpace: Int,
        rotationDegrees: Int,
        hardwareBuffer: HardwareBuffer,
    ) {
        assertMainThread()
        SimulatorBridge.postHardwareBuffer(width, height, dataSpace, rotationDegrees, hardwareBuffer)
        SimulatorBridge.draw()
    }

    fun postRgba(width: Int, height: Int, pixels: ByteBuffer) {
        assertMainThread()
        SimulatorBridge.postRgba(width, height, pixels)
        SimulatorBridge.draw()
    }

    fun postSettings(jsonString: String) {
        assertMainThread()
        SimulatorBridge.postSettings(jsonString)
        SimulatorBridge.draw()
    }

    fun querySettings(): String {
        assertMainThread()
        return SimulatorBridge.querySettings()
    }

    private fun assertMainThread() {
        assert(Looper.getMainLooper().isCurrentThread) { "Called from non-UI thread" }
    }
}
