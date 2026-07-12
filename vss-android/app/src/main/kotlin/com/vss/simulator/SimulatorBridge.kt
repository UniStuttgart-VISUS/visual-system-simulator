package com.vss.simulator

import android.content.res.AssetManager
import android.hardware.HardwareBuffer
import android.util.Log
import android.view.Surface
import java.nio.ByteBuffer

/** Bridge for accessing the simulator's native library. */
object SimulatorBridge {
    private const val LOG_TAG = "SimulatorBridge"

    private var libraryLoaded = false

    init {
        try {
            Log.d(LOG_TAG, "Loading native library...")
            System.loadLibrary("vss_android")
            libraryLoaded = true
            Log.i(LOG_TAG, "Loading native library: successful")
        } catch (error: UnsatisfiedLinkError) {
            Log.e(LOG_TAG, "Loading native library: failed", error)
        }
    }

    @JvmStatic
    private external fun nativeCreate(surface: Surface, assetManager: AssetManager)

    @JvmStatic
    private external fun nativeDestroy()

    @JvmStatic
    private external fun nativeResize(width: Int, height: Int)

    @JvmStatic
    private external fun nativeDraw()

    @JvmStatic
    private external fun nativePostHardwareBuffer(
        width: Int,
        height: Int,
        dataSpace: Int,
        rotationDegrees: Int,
        hardwareBuffer: HardwareBuffer,
    )

    @JvmStatic
    private external fun nativePostRgba(width: Int, height: Int, pixels: ByteBuffer)

    @JvmStatic
    private external fun nativePostSettings(jsonString: String)

    @JvmStatic
    private external fun nativeQuerySettings(): String

    @JvmStatic
    private external fun nativeCatalog(locale: String): String

    @JvmStatic
    private external fun nativeComposeSettings(
        locale: String,
        activeProfiles: String,
        manualOverrides: String,
    ): String

    fun hasLoadedLibrary(): Boolean = libraryLoaded

    fun create(surface: Surface, assetManager: AssetManager) {
        assert(libraryLoaded) { "Native library not loaded" }
        Log.v(LOG_TAG, "Creating simulator")
        nativeCreate(surface, assetManager)
    }

    fun destroy() {
        assert(libraryLoaded) { "Native library not loaded" }
        Log.v(LOG_TAG, "Destroying simulator")
        nativeDestroy()
    }

    fun resize(width: Int, height: Int) {
        assert(libraryLoaded) { "Native library not loaded" }
        Log.v(LOG_TAG, "Resizing simulation to ${width}x$height")
        nativeResize(width, height)
    }

    fun draw() {
        assert(libraryLoaded) { "Native library not loaded" }
        nativeDraw()
    }

    fun postHardwareBuffer(
        width: Int,
        height: Int,
        dataSpace: Int,
        rotationDegrees: Int,
        hardwareBuffer: HardwareBuffer,
    ) {
        assert(libraryLoaded) { "Native library not loaded" }
        nativePostHardwareBuffer(width, height, dataSpace, rotationDegrees, hardwareBuffer)
    }

    fun postRgba(width: Int, height: Int, pixels: ByteBuffer) {
        assert(libraryLoaded) { "Native library not loaded" }
        nativePostRgba(width, height, pixels)
    }

    fun postSettings(jsonString: String) {
        assert(libraryLoaded) { "Native library not loaded" }
        Log.v(LOG_TAG, "Posting simulator settings: $jsonString")
        nativePostSettings(jsonString)
    }

    fun querySettings(): String {
        assert(libraryLoaded) { "Native library not loaded" }
        Log.v(LOG_TAG, "Querying simulator settings")
        return nativeQuerySettings()
    }

    fun catalog(locale: String): String {
        assert(libraryLoaded) { "Native library not loaded" }
        return nativeCatalog(locale)
    }

    fun composeSettings(
        locale: String,
        activeProfiles: String,
        manualOverrides: String,
    ): String {
        assert(libraryLoaded) { "Native library not loaded" }
        return nativeComposeSettings(locale, activeProfiles, manualOverrides)
    }
}
