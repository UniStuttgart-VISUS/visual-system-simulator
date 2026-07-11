package com.vss

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.graphics.ImageFormat
import android.graphics.Point
import android.hardware.HardwareBuffer
import android.hardware.SyncFence
import android.hardware.camera2.CameraAccessException
import android.hardware.camera2.CameraCaptureSession
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraDevice
import android.hardware.camera2.CameraManager
import android.hardware.camera2.CaptureRequest
import android.hardware.camera2.params.OutputConfiguration
import android.hardware.camera2.params.SessionConfiguration
import android.media.Image
import android.media.ImageReader
import android.os.Build
import android.os.Handler
import android.util.Log
import android.util.Size
import android.view.Surface
import android.view.WindowManager
import androidx.core.app.ActivityCompat
import java.io.IOException
import kotlin.math.abs
import kotlin.math.atan
import kotlin.math.max
import kotlin.math.min
import kotlin.math.roundToInt

class CameraFrameSource(
    private val context: Context,
    private val delegate: CameraDelegate,
) {
    private var imageReader: ImageReader? = null
    private var cameraDevice: CameraDevice? = null
    private var captureSession: CameraCaptureSession? = null

    init {
        setupCamera()
    }

    private fun checkCameraPermission(): Boolean {
        if (
            ActivityCompat.checkSelfPermission(context, Manifest.permission.CAMERA) ==
            PackageManager.PERMISSION_DENIED
        ) {
            delegate.onCameraPermissionDenied()
            return false
        }
        return true
    }

    @Suppress("DEPRECATION")
    private fun getScreenSize(): Size {
        val windowManager = context.getSystemService(Context.WINDOW_SERVICE) as WindowManager
        val size = Point()
        windowManager.defaultDisplay.getSize(size)
        return Size(size.x, size.y)
    }

    private fun getBestResolution(cameraSizes: List<Size>, screenSize: Size): Size? {
        val minScreen = min(screenSize.width, screenSize.height)
        val maxScreen = max(screenSize.width, screenSize.height)
        var bestSize: Size? = null
        var bestDiff = Int.MAX_VALUE
        for (size in cameraSizes) {
            val diffA = abs(min(size.width, size.height) - minScreen)
            val diffB = abs(max(size.width, size.height) - maxScreen)
            if (diffA < bestDiff || diffB < bestDiff) {
                bestDiff = min(diffA, diffB)
                bestSize = size
            }
        }
        return bestSize
    }

    @Throws(CameraAccessException::class)
    private fun selectCamera(manager: CameraManager, screenSize: Size): CameraSelection? {
        var best: CameraSelection? = null
        for (cameraId in manager.cameraIdList) {
            val characteristics = manager.getCameraCharacteristics(cameraId)
            val facing = characteristics[CameraCharacteristics.LENS_FACING]
            if (facing == null || facing != CameraCharacteristics.LENS_FACING_BACK) {
                continue
            }

            val map = characteristics[CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP]
                ?: continue
            val outputSizes = map.getOutputSizes(CAMERA_IMAGE_FORMAT)?.toList() ?: continue
            if (outputSizes.isEmpty()) {
                Log.w(LOG_TAG, "No YUV_420_888 output sizes for camera $cameraId")
                continue
            }

            val bestSize = getBestResolution(outputSizes, screenSize) ?: continue
            val normalFovError = normalCameraFovError(characteristics)
            val logicalCamera = hasCapability(
                characteristics,
                CameraCharacteristics.REQUEST_AVAILABLE_CAPABILITIES_LOGICAL_MULTI_CAMERA,
            )
            val zoomRatioSupportsOne = zoomRatioSupportsOne(characteristics)
            var score = 1000
            if (logicalCamera) score += 500
            if (zoomRatioSupportsOne) score += 300
            if (normalFovError < Float.MAX_VALUE) {
                score += max(0, (300 - normalFovError * 10).roundToInt())
            }

            Log.i(
                LOG_TAG,
                "Camera candidate $cameraId: score=$score, size=$bestSize, " +
                    "logical=$logicalCamera, zoom1=$zoomRatioSupportsOne, " +
                    "normalFovError=$normalFovError",
            )

            val candidate = CameraSelection(
                cameraId = cameraId,
                characteristics = characteristics,
                size = bestSize,
                sensorOrientationDegrees = sensorOrientation(characteristics),
                zoomRatioSupportsOne = zoomRatioSupportsOne,
                score = score,
            )
            if (best == null || candidate.score > best.score) {
                best = candidate
            }
        }
        return best
    }

    private fun hasCapability(
        characteristics: CameraCharacteristics,
        capability: Int,
    ): Boolean = characteristics[CameraCharacteristics.REQUEST_AVAILABLE_CAPABILITIES]
        ?.contains(capability) == true

    private fun zoomRatioSupportsOne(characteristics: CameraCharacteristics): Boolean =
        characteristics[CameraCharacteristics.CONTROL_ZOOM_RATIO_RANGE]?.contains(1.0f) == true

    private fun normalCameraFovError(characteristics: CameraCharacteristics): Float {
        val sensorSize = characteristics[CameraCharacteristics.SENSOR_INFO_PHYSICAL_SIZE]
            ?: return Float.MAX_VALUE
        val focalLengths = characteristics[CameraCharacteristics.LENS_INFO_AVAILABLE_FOCAL_LENGTHS]
            ?: return Float.MAX_VALUE
        if (focalLengths.isEmpty()) return Float.MAX_VALUE

        var bestError = Float.MAX_VALUE
        for (focalLength in focalLengths) {
            if (focalLength <= 0.0f) continue
            val fov = Math.toDegrees(
                2.0 * atan(sensorSize.width / (2.0 * focalLength)).toDouble(),
            ).toFloat()
            bestError = min(bestError, abs(fov - TARGET_HORIZONTAL_FOV_DEGREES))
        }
        return bestError
    }

    private fun sensorOrientation(characteristics: CameraCharacteristics): Int =
        characteristics[CameraCharacteristics.SENSOR_ORIENTATION] ?: 0

    @Suppress("DEPRECATION")
    private fun displayRotationDegrees(): Int {
        val windowManager = context.getSystemService(Context.WINDOW_SERVICE) as WindowManager
        return when (windowManager.defaultDisplay.rotation) {
            Surface.ROTATION_90 -> 90
            Surface.ROTATION_180 -> 180
            Surface.ROTATION_270 -> 270
            else -> 0
        }
    }

    private fun setupCamera() {
        if (!checkCameraPermission()) return

        val manager = context.getSystemService(Context.CAMERA_SERVICE) as CameraManager
        try {
            val screenSize = getScreenSize()
            val selection = selectCamera(manager, screenSize)
            if (selection == null) {
                Log.e(LOG_TAG, "No suitable back-facing YUV_420_888 camera found")
                return
            }

            Log.i(
                LOG_TAG,
                "Using hardware-buffer camera ${selection.cameraId} at ${selection.size} " +
                    "(sensor orientation is ${selection.sensorOrientationDegrees}, " +
                    "display rotation is ${displayRotationDegrees()}) " +
                    "(screen resolution is $screenSize)",
            )

            manager.openCamera(
                selection.cameraId,
                context.mainExecutor,
                object : CameraDevice.StateCallback() {
                    override fun onOpened(cameraDevice: CameraDevice) {
                        Log.i(LOG_TAG, "Device opened")
                        setupCameraSession(cameraDevice, selection)
                    }

                    override fun onDisconnected(cameraDevice: CameraDevice) {
                        Log.i(LOG_TAG, "Device disconnected")
                    }

                    override fun onError(cameraDevice: CameraDevice, error: Int) {
                        Log.e(LOG_TAG, "Device error ($error)")
                    }
                },
            )
        } catch (error: CameraAccessException) {
            Log.e(LOG_TAG, "Setting up camera failed", error)
        }
    }

    private fun setupCameraSession(cameraDevice: CameraDevice, selection: CameraSelection) {
        val width = selection.size.width
        val height = selection.size.height
        val reader = createHardwareBufferImageReader(width, height, CAMERA_IMAGE_MAX_IMAGES)
        imageReader = reader
        reader.setOnImageAvailableListener({ availableReader ->
            val image = availableReader.acquireLatestImage() ?: return@setOnImageAvailableListener
            var hardwareBuffer: HardwareBuffer? = null
            try {
                waitForAcquireFence(image)
                hardwareBuffer = image.hardwareBuffer
                hardwareBuffer?.let {
                    delegate.onFrameAvailable(
                        width,
                        height,
                        image.dataSpace,
                        selection.outputRotationDegrees(displayRotationDegrees()),
                        it,
                    )
                }
            } finally {
                hardwareBuffer?.close()
                image.close()
            }
        }, null)

        try {
            cameraDevice.createCaptureSession(
                SessionConfiguration(
                    SessionConfiguration.SESSION_REGULAR,
                    listOf(OutputConfiguration(reader.surface)),
                    context.mainExecutor,
                    object : CameraCaptureSession.StateCallback() {
                        override fun onConfigured(session: CameraCaptureSession) {
                            captureSession = session
                            startRepeatingRequest(cameraDevice, session, selection)
                        }

                        override fun onConfigureFailed(session: CameraCaptureSession) {
                            Log.e(SESSION_LOG_TAG, "Configure failed")
                        }
                    },
                ),
            )
        } catch (error: CameraAccessException) {
            Log.e(SESSION_LOG_TAG, "Creating session", error)
        }

        this.cameraDevice = cameraDevice
    }

    private fun startRepeatingRequest(
        cameraDevice: CameraDevice,
        session: CameraCaptureSession,
        selection: CameraSelection,
    ) {
        try {
            val requestBuilder = cameraDevice.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW)
            requestBuilder.addTarget(requireNotNull(imageReader).surface)
            requestBuilder.set(
                CaptureRequest.CONTROL_AF_MODE,
                CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_PICTURE,
            )
            if (selection.zoomRatioSupportsOne) {
                requestBuilder.set(CaptureRequest.CONTROL_ZOOM_RATIO, 1.0f)
            }
            session.setRepeatingRequest(
                requestBuilder.build(),
                object : CameraCaptureSession.CaptureCallback() {},
                Handler(context.mainLooper),
            )
        } catch (error: CameraAccessException) {
            Log.e(SESSION_LOG_TAG, "Configure failed", error)
        }
    }

    private fun createHardwareBufferImageReader(
        width: Int,
        height: Int,
        maxImages: Int,
    ): ImageReader {
        val usage = HardwareBuffer.USAGE_GPU_SAMPLED_IMAGE
        Log.i(LOG_TAG, "Creating GPU-sampled ImageReader with usage 0x${usage.toString(16)}")
        return ImageReader.newInstance(width, height, CAMERA_IMAGE_FORMAT, maxImages, usage)
    }

    private fun waitForAcquireFence(image: Image) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return

        try {
            image.fence.use { fence: SyncFence? ->
                if (fence != null && fence.isValid && !fence.awaitForever()) {
                    Log.w(LOG_TAG, "Camera acquire fence reported an error")
                }
            }
        } catch (error: IOException) {
            Log.w(
                LOG_TAG,
                "Cannot get camera acquire fence; continuing without explicit wait",
                error,
            )
        }
    }

    fun close() {
        captureSession?.close()
        captureSession = null
        cameraDevice?.close()
        cameraDevice = null
        imageReader?.close()
        imageReader = null
    }

    interface CameraDelegate {
        fun onCameraPermissionDenied()

        fun onFrameAvailable(
            width: Int,
            height: Int,
            dataSpace: Int,
            rotationDegrees: Int,
            hardwareBuffer: HardwareBuffer,
        )
    }

    private data class CameraSelection(
        val cameraId: String,
        val characteristics: CameraCharacteristics,
        val size: Size,
        val sensorOrientationDegrees: Int,
        val zoomRatioSupportsOne: Boolean,
        val score: Int,
    ) {
        fun outputRotationDegrees(displayRotationDegrees: Int): Int =
            ((sensorOrientationDegrees - displayRotationDegrees) % 360 + 360) % 360
    }

    private companion object {
        const val LOG_TAG = "CameraFrameSource"
        const val SESSION_LOG_TAG = "CameraFrameSourceSession"
        const val CAMERA_IMAGE_FORMAT = ImageFormat.YUV_420_888
        const val CAMERA_IMAGE_MAX_IMAGES = 2
        const val TARGET_HORIZONTAL_FOV_DEGREES = 75.0f
    }
}
