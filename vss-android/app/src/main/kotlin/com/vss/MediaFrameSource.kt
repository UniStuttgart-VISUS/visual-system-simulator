package com.vss

import android.content.Context
import android.graphics.Bitmap
import android.graphics.ImageDecoder
import android.graphics.ImageFormat
import android.hardware.HardwareBuffer
import android.hardware.SyncFence
import android.media.Image
import android.media.ImageReader
import android.media.MediaCodec
import android.media.MediaExtractor
import android.media.MediaFormat
import android.net.Uri
import android.os.Build
import android.os.Handler
import android.os.HandlerThread
import android.os.Looper
import android.util.Log
import java.io.IOException
import java.nio.ByteBuffer
import java.util.concurrent.CountDownLatch
import kotlin.math.max

class MediaFrameSource(context: Context, private val controller: SimulatorController) {
    private val context = context.applicationContext
    private val mainHandler = Handler(Looper.getMainLooper())
    private val hardwareFrameLock = Any()

    private var workerThread: Thread? = null

    @Volatile
    private var cancelled = false

    private var pendingHardwareFrame: HardwareFrame? = null
    private var hardwareFramePostScheduled = false

    fun start(uri: Uri, mimeType: String?) {
        close()
        cancelled = false
        workerThread = Thread({ decode(uri, mimeType) }, "VSS media").also { it.start() }
    }

    fun close() {
        cancelled = true
        closePendingHardwareFrame()
        workerThread?.interrupt()
        workerThread = null
    }

    private fun decode(uri: Uri, mimeType: String?) {
        try {
            val resolvedMimeType = resolveMimeType(uri, mimeType)
            if (resolvedMimeType?.startsWith("video/") == true) {
                decodeVideo(uri)
            } else if (!decodeImage(uri)) {
                Log.w(LOG_TAG, "Image could not be decoded: $uri")
            }
        } catch (error: Exception) {
            Log.e(LOG_TAG, "Cannot decode media $uri", error)
        }
    }

    private fun resolveMimeType(uri: Uri, mimeType: String?): String? {
        if (mimeType != null && mimeType != "*/*") return mimeType
        return context.contentResolver.getType(uri)
    }

    @Throws(IOException::class)
    private fun decodeImage(uri: Uri): Boolean {
        val bitmap: Bitmap
        try {
            val source = ImageDecoder.createSource(context.contentResolver, uri)
            bitmap = ImageDecoder.decodeBitmap(source) { decoder, info, _ ->
                decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
                val width = info.size.width
                val height = info.size.height
                val sampleSize = sampleSize(width, height)
                if (sampleSize > 1) {
                    decoder.setTargetSize(
                        max(1, width / sampleSize),
                        max(1, height / sampleSize),
                    )
                }
            }
        } catch (error: IOException) {
            Log.w(LOG_TAG, "Hardware image decode failed", error)
            return false
        } catch (error: RuntimeException) {
            Log.w(LOG_TAG, "Hardware image decode failed", error)
            return false
        } catch (error: OutOfMemoryError) {
            Log.w(LOG_TAG, "Hardware image decode ran out of memory", error)
            return false
        }

        val width = bitmap.width
        val height = bitmap.height
        val pixels = ByteBuffer.allocateDirect(width * height * 4)
        bitmap.copyPixelsToBuffer(pixels)
        pixels.rewind()
        bitmap.recycle()
        mainHandler.post { postRgbaWhenSurfaceIsReady(width, height, pixels) }
        return true
    }

    private fun postRgbaWhenSurfaceIsReady(width: Int, height: Int, pixels: ByteBuffer) {
        if (cancelled) return
        if (!controller.isReady()) {
            mainHandler.postDelayed(
                { postRgbaWhenSurfaceIsReady(width, height, pixels) },
                SURFACE_RETRY_DELAY_MS,
            )
            return
        }
        controller.postRgba(width, height, pixels)
    }

    private fun sampleSize(width: Int, height: Int): Int {
        var sampleSize = 1
        val edge = max(width, height)
        while (edge / sampleSize > MAX_IMAGE_EDGE) {
            sampleSize *= 2
        }
        return sampleSize
    }

    @Throws(IOException::class)
    private fun decodeVideo(uri: Uri) {
        val extractor = MediaExtractor()
        var decoder: MediaCodec? = null
        var imageReader: ImageReader? = null
        var imageThread: HandlerThread? = null
        var decoderStarted = false
        try {
            extractor.setDataSource(context, uri, null)
            val trackIndex = selectVideoTrack(extractor)
            if (trackIndex < 0) {
                Log.w(LOG_TAG, "Selected media contains no video track: $uri")
                return
            }

            extractor.selectTrack(trackIndex)
            val format = extractor.getTrackFormat(trackIndex)
            val mimeType = format.getString(MediaFormat.KEY_MIME)
            if (mimeType == null) {
                Log.w(LOG_TAG, "Selected video track has no MIME type: $uri")
                return
            }

            val width = format.getInteger(MediaFormat.KEY_WIDTH)
            val height = format.getInteger(MediaFormat.KEY_HEIGHT)
            val rotationDegrees = videoRotationDegrees(format)

            imageThread = HandlerThread("VSS media images").also { it.start() }
            val imageHandler = Handler(imageThread.looper)
            imageReader = ImageReader.newInstance(
                width,
                height,
                ImageFormat.PRIVATE,
                MAX_VIDEO_IMAGES,
            )
            val frameReader = imageReader
            imageReader.setOnImageAvailableListener({ reader ->
                val image = try {
                    reader.acquireLatestImage()
                } catch (error: IllegalStateException) {
                    Log.w(
                        LOG_TAG,
                        "Dropping decoded frame because the media ImageReader is full",
                        error,
                    )
                    return@setOnImageAvailableListener
                } ?: return@setOnImageAvailableListener

                var hardwareBuffer: HardwareBuffer? = null
                try {
                    waitForAcquireFence(image)
                    hardwareBuffer = image.hardwareBuffer
                    if (hardwareBuffer == null) {
                        image.close()
                        return@setOnImageAvailableListener
                    }
                    submitHardwareFrame(
                        width,
                        height,
                        image.dataSpace,
                        rotationDegrees,
                        hardwareBuffer,
                        image::close,
                    )
                } catch (error: RuntimeException) {
                    Log.w(LOG_TAG, "Cannot forward decoded video frame", error)
                    hardwareBuffer?.close()
                    image.close()
                }
            }, imageHandler)

            decoder = MediaCodec.createDecoderByType(mimeType)
            decoder.configure(format, frameReader.surface, null, 0)
            decoder.start()
            decoderStarted = true
            drainVideo(extractor, decoder, imageHandler)
        } finally {
            decoder?.let {
                if (decoderStarted) it.stop()
                it.release()
            }
            imageReader?.close()
            imageThread?.quitSafely()
            extractor.release()
        }
    }

    private fun selectVideoTrack(extractor: MediaExtractor): Int {
        for (index in 0 until extractor.trackCount) {
            val mimeType = extractor.getTrackFormat(index).getString(MediaFormat.KEY_MIME)
            if (mimeType?.startsWith("video/") == true) return index
        }
        return -1
    }

    private fun videoRotationDegrees(format: MediaFormat): Int {
        if (!format.containsKey(MediaFormat.KEY_ROTATION)) return 0
        return normalizeRotationDegrees(format.getInteger(MediaFormat.KEY_ROTATION))
    }

    private fun normalizeRotationDegrees(rotationDegrees: Int): Int {
        val normalized = ((rotationDegrees % 360) + 360) % 360
        return when (normalized) {
            in 45..134 -> 90
            in 135..224 -> 180
            in 225..314 -> 270
            else -> 0
        }
    }

    private fun drainVideo(
        extractor: MediaExtractor,
        decoder: MediaCodec,
        imageHandler: Handler,
    ) {
        val bufferInfo = MediaCodec.BufferInfo()
        var inputEnded = false
        var playbackStartMs = -1L

        while (!cancelled) {
            if (!inputEnded) {
                val inputIndex = decoder.dequeueInputBuffer(CODEC_TIMEOUT_US)
                if (inputIndex >= 0) {
                    val inputBuffer = decoder.getInputBuffer(inputIndex)
                    inputBuffer?.clear()
                    val sampleSize = inputBuffer?.let { extractor.readSampleData(it, 0) } ?: -1
                    if (sampleSize < 0) {
                        decoder.queueInputBuffer(
                            inputIndex,
                            0,
                            0,
                            0,
                            MediaCodec.BUFFER_FLAG_END_OF_STREAM,
                        )
                        inputEnded = true
                    } else {
                        decoder.queueInputBuffer(
                            inputIndex,
                            0,
                            sampleSize,
                            extractor.sampleTime,
                            extractor.sampleFlags,
                        )
                        extractor.advance()
                    }
                }
            }

            val outputIndex = decoder.dequeueOutputBuffer(bufferInfo, CODEC_TIMEOUT_US)
            if (outputIndex >= 0) {
                if (playbackStartMs < 0) {
                    playbackStartMs =
                        System.currentTimeMillis() - bufferInfo.presentationTimeUs / 1000L
                }
                waitUntilPresentationTime(playbackStartMs, bufferInfo.presentationTimeUs)
                val endOfStream =
                    bufferInfo.flags and MediaCodec.BUFFER_FLAG_END_OF_STREAM != 0
                decoder.releaseOutputBuffer(outputIndex, !endOfStream)
                if (endOfStream) {
                    waitForPendingVideoFrames(imageHandler)
                    decoder.flush()
                    extractor.seekTo(0, MediaExtractor.SEEK_TO_PREVIOUS_SYNC)
                    inputEnded = false
                    playbackStartMs = -1L
                }
            } else if (outputIndex == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                Log.d(LOG_TAG, "Video decoder output format changed: ${decoder.outputFormat}")
            }
        }
    }

    private fun waitForPendingVideoFrames(imageHandler: Handler) {
        val imagesDrained = CountDownLatch(1)
        val framesSubmitted = CountDownLatch(1)
        imageHandler.post {
            imagesDrained.countDown()
            mainHandler.post { framesSubmitted.countDown() }
        }
        try {
            imagesDrained.await()
            framesSubmitted.await()
        } catch (error: InterruptedException) {
            Thread.currentThread().interrupt()
            cancelled = true
        }
    }

    private fun waitUntilPresentationTime(playbackStartMs: Long, presentationTimeUs: Long) {
        val delayMs = playbackStartMs + presentationTimeUs / 1000L - System.currentTimeMillis()
        if (delayMs <= 0) return
        try {
            Thread.sleep(delayMs)
        } catch (error: InterruptedException) {
            Thread.currentThread().interrupt()
            cancelled = true
        }
    }

    private fun submitHardwareFrame(
        width: Int,
        height: Int,
        dataSpace: Int,
        rotationDegrees: Int,
        hardwareBuffer: HardwareBuffer,
        releaseOwner: () -> Unit,
    ) {
        postHardwareFrameWhenSurfaceIsReady(
            HardwareFrame(
                width,
                height,
                dataSpace,
                rotationDegrees,
                hardwareBuffer,
                releaseOwner,
            ),
        )
    }

    private fun postHardwareFrameWhenSurfaceIsReady(frame: HardwareFrame) {
        var shouldPost = false
        synchronized(hardwareFrameLock) {
            pendingHardwareFrame?.close()
            pendingHardwareFrame = frame
            if (!hardwareFramePostScheduled) {
                hardwareFramePostScheduled = true
                shouldPost = true
            }
        }
        if (shouldPost) {
            mainHandler.post(::postPendingHardwareFrameOnMainThread)
        }
    }

    private fun postPendingHardwareFrameOnMainThread() {
        val frame = synchronized(hardwareFrameLock) {
            pendingHardwareFrame.also {
                pendingHardwareFrame = null
                if (it == null) hardwareFramePostScheduled = false
            }
        } ?: return

        if (cancelled) {
            frame.close()
            synchronized(hardwareFrameLock) {
                hardwareFramePostScheduled = false
            }
            return
        }
        if (!controller.isReady()) {
            mainHandler.postDelayed(
                ::postPendingHardwareFrameOnMainThread,
                SURFACE_RETRY_DELAY_MS,
            )
            synchronized(hardwareFrameLock) {
                if (pendingHardwareFrame != null) {
                    frame.close()
                } else {
                    pendingHardwareFrame = frame
                }
            }
            return
        }
        try {
            controller.postHardwareBuffer(
                frame.width,
                frame.height,
                frame.dataSpace,
                frame.rotationDegrees,
                frame.hardwareBuffer,
            )
        } finally {
            frame.close()
        }

        val shouldPostAgain = synchronized(hardwareFrameLock) {
            (pendingHardwareFrame != null).also { hardwareFramePostScheduled = it }
        }
        if (shouldPostAgain) {
            mainHandler.post(::postPendingHardwareFrameOnMainThread)
        }
    }

    private fun closePendingHardwareFrame() {
        synchronized(hardwareFrameLock) {
            pendingHardwareFrame?.close()
            pendingHardwareFrame = null
            hardwareFramePostScheduled = false
        }
    }

    private fun waitForAcquireFence(image: Image) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) return

        try {
            image.fence.use { fence: SyncFence? ->
                if (fence != null && fence.isValid && !fence.awaitForever()) {
                    Log.w(LOG_TAG, "Media acquire fence reported an error")
                }
            }
        } catch (error: IOException) {
            Log.w(
                LOG_TAG,
                "Cannot get media acquire fence; continuing without explicit wait",
                error,
            )
        }
    }

    private class HardwareFrame(
        val width: Int,
        val height: Int,
        val dataSpace: Int,
        val rotationDegrees: Int,
        val hardwareBuffer: HardwareBuffer,
        val releaseOwner: () -> Unit,
    ) {
        fun close() {
            try {
                hardwareBuffer.close()
            } finally {
                releaseOwner()
            }
        }
    }

    private companion object {
        const val LOG_TAG = "MediaFrameSource"
        const val MAX_IMAGE_EDGE = 1920
        const val MAX_VIDEO_IMAGES = 6
        const val SURFACE_RETRY_DELAY_MS = 16L
        const val CODEC_TIMEOUT_US = 10_000L
    }
}
