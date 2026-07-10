package com.vss;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.ImageDecoder;
import android.graphics.ImageFormat;
import android.hardware.DataSpace;
import android.hardware.HardwareBuffer;
import android.hardware.SyncFence;
import android.media.Image;
import android.media.ImageReader;
import android.media.MediaCodec;
import android.media.MediaExtractor;
import android.media.MediaFormat;
import android.net.Uri;
import android.os.Build;
import android.os.Handler;
import android.os.HandlerThread;
import android.os.Looper;
import android.util.Log;

import com.vss.simulator.SimulatorSurfaceView;

import java.io.IOException;
import java.nio.ByteBuffer;
import java.util.concurrent.CountDownLatch;

class MediaFrameSource {
    private static final String LOG_TAG = "MediaFrameSource";
    private static final int MAX_IMAGE_EDGE = 1920;
    private static final int MAX_VIDEO_IMAGES = 6;
    private static final int SURFACE_RETRY_DELAY_MS = 16;
    private static final long CODEC_TIMEOUT_US = 10_000L;

    private final Context context;
    private final SimulatorSurfaceView simulatorView;
    private final Handler mainHandler = new Handler(Looper.getMainLooper());
    private final Object hardwareFrameLock = new Object();

    private Thread workerThread;
    private volatile boolean cancelled;
    private HardwareFrame pendingHardwareFrame;
    private boolean hardwareFramePostScheduled;

    MediaFrameSource(Context context, SimulatorSurfaceView simulatorView) {
        this.context = context.getApplicationContext();
        this.simulatorView = simulatorView;
    }

    void start(Uri uri, String mimeType) {
        close();
        cancelled = false;
        workerThread = new Thread(() -> decode(uri, mimeType), "VSS media");
        workerThread.start();
    }

    void close() {
        cancelled = true;
        closePendingHardwareFrame();
        if (workerThread != null) {
            workerThread.interrupt();
            workerThread = null;
        }
    }

    private void decode(Uri uri, String mimeType) {
        try {
            String resolvedMimeType = resolveMimeType(uri, mimeType);
            if (resolvedMimeType != null && resolvedMimeType.startsWith("video/")) {
                decodeVideo(uri);
            } else if (!decodeImage(uri)) {
                Log.w(LOG_TAG, "Image could not be decoded: " + uri);
            }
        } catch (Exception err) {
            Log.e(LOG_TAG, "Cannot decode media " + uri, err);
        }
    }

    private String resolveMimeType(Uri uri, String mimeType) {
        if (mimeType != null && !mimeType.equals("*/*")) {
            return mimeType;
        }
        return context.getContentResolver().getType(uri);
    }

    private boolean decodeImage(Uri uri) throws IOException {
        Bitmap bitmap;
        try {
            ImageDecoder.Source source = ImageDecoder.createSource(context.getContentResolver(), uri);
            bitmap = ImageDecoder.decodeBitmap(source, (decoder, info, src) -> {
                decoder.setAllocator(ImageDecoder.ALLOCATOR_SOFTWARE);
                int width = info.getSize().getWidth();
                int height = info.getSize().getHeight();
                int sampleSize = sampleSize(width, height);
                if (sampleSize > 1) {
                    decoder.setTargetSize(Math.max(1, width / sampleSize), Math.max(1, height / sampleSize));
                }
            });
        } catch (IOException | RuntimeException err) {
            Log.w(LOG_TAG, "Hardware image decode failed", err);
            return false;
        } catch (OutOfMemoryError err) {
            Log.w(LOG_TAG, "Hardware image decode ran out of memory", err);
            return false;
        }

        int width = bitmap.getWidth();
        int height = bitmap.getHeight();
        ByteBuffer pixels = ByteBuffer.allocateDirect(width * height * 4);
        bitmap.copyPixelsToBuffer(pixels);
        pixels.rewind();
        bitmap.recycle();
        mainHandler.post(() -> postRgbaWhenSurfaceIsReady(width, height, pixels));
        return true;
    }

    private void postRgbaWhenSurfaceIsReady(int width, int height, ByteBuffer pixels) {
        if (cancelled) {
            return;
        }
        if (!simulatorView.getHolder().getSurface().isValid()) {
            mainHandler.postDelayed(
                    () -> postRgbaWhenSurfaceIsReady(width, height, pixels),
                    SURFACE_RETRY_DELAY_MS
            );
            return;
        }
        simulatorView.postRgba(width, height, pixels);
    }

    private int sampleSize(int width, int height) {
        int sampleSize = 1;
        int edge = Math.max(width, height);
        while (edge / sampleSize > MAX_IMAGE_EDGE) {
            sampleSize *= 2;
        }
        return sampleSize;
    }

    private void decodeVideo(Uri uri) throws IOException {
        MediaExtractor extractor = new MediaExtractor();
        MediaCodec decoder = null;
        ImageReader imageReader = null;
        HandlerThread imageThread = null;
        boolean decoderStarted = false;
        try {
            extractor.setDataSource(context, uri, null);
            int trackIndex = selectVideoTrack(extractor);
            if (trackIndex < 0) {
                Log.w(LOG_TAG, "Selected media contains no video track: " + uri);
                return;
            }

            extractor.selectTrack(trackIndex);
            MediaFormat format = extractor.getTrackFormat(trackIndex);
            String mimeType = format.getString(MediaFormat.KEY_MIME);
            if (mimeType == null) {
                Log.w(LOG_TAG, "Selected video track has no MIME type: " + uri);
                return;
            }

            int width = format.getInteger(MediaFormat.KEY_WIDTH);
            int height = format.getInteger(MediaFormat.KEY_HEIGHT);
            int rotationDegrees = videoRotationDegrees(format);

            imageThread = new HandlerThread("VSS media images");
            imageThread.start();
            Handler imageHandler = new Handler(imageThread.getLooper());
            imageReader = ImageReader.newInstance(width, height, ImageFormat.PRIVATE, MAX_VIDEO_IMAGES);
            final ImageReader frameReader = imageReader;
            final int frameWidth = width;
            final int frameHeight = height;
            final int frameRotationDegrees = rotationDegrees;
            imageReader.setOnImageAvailableListener(reader -> {
                Image image;
                try {
                    image = reader.acquireLatestImage();
                } catch (IllegalStateException err) {
                    Log.w(LOG_TAG, "Dropping decoded frame because the media ImageReader is full", err);
                    return;
                }
                if (image == null) {
                    return;
                }

                HardwareBuffer hardwareBuffer = null;
                try {
                    waitForAcquireFence(image);
                    hardwareBuffer = image.getHardwareBuffer();
                    if (hardwareBuffer == null) {
                        image.close();
                        return;
                    }
                    submitHardwareFrame(
                            frameWidth,
                            frameHeight,
                            image.getDataSpace(),
                            frameRotationDegrees,
                            hardwareBuffer,
                            image::close
                    );
                } catch (RuntimeException err) {
                    Log.w(LOG_TAG, "Cannot forward decoded video frame", err);
                    if (hardwareBuffer != null) {
                        hardwareBuffer.close();
                    }
                    image.close();
                }
            }, imageHandler);

            decoder = MediaCodec.createDecoderByType(mimeType);
            decoder.configure(format, frameReader.getSurface(), null, 0);
            decoder.start();
            decoderStarted = true;
            drainVideo(extractor, decoder, imageHandler);
        } finally {
            if (decoder != null) {
                if (decoderStarted) {
                    decoder.stop();
                }
                decoder.release();
            }
            if (imageReader != null) {
                imageReader.close();
            }
            if (imageThread != null) {
                imageThread.quitSafely();
            }
            extractor.release();
        }
    }

    private int selectVideoTrack(MediaExtractor extractor) {
        for (int i = 0; i < extractor.getTrackCount(); i++) {
            MediaFormat format = extractor.getTrackFormat(i);
            String mimeType = format.getString(MediaFormat.KEY_MIME);
            if (mimeType != null && mimeType.startsWith("video/")) {
                return i;
            }
        }
        return -1;
    }

    private int videoRotationDegrees(MediaFormat format) {
        if (!format.containsKey(MediaFormat.KEY_ROTATION)) {
            return 0;
        }
        return normalizeRotationDegrees(format.getInteger(MediaFormat.KEY_ROTATION));
    }

    private int normalizeRotationDegrees(int rotationDegrees) {
        int normalized = ((rotationDegrees % 360) + 360) % 360;
        if (normalized >= 45 && normalized <= 134) {
            return 90;
        }
        if (normalized >= 135 && normalized <= 224) {
            return 180;
        }
        if (normalized >= 225 && normalized <= 314) {
            return 270;
        }
        return 0;
    }

    private void drainVideo(MediaExtractor extractor, MediaCodec decoder, Handler imageHandler) {
        MediaCodec.BufferInfo bufferInfo = new MediaCodec.BufferInfo();
        boolean inputEnded = false;
        long playbackStartMs = -1L;

        while (!cancelled) {
            if (!inputEnded) {
                int inputIndex = decoder.dequeueInputBuffer(CODEC_TIMEOUT_US);
                if (inputIndex >= 0) {
                    ByteBuffer inputBuffer = decoder.getInputBuffer(inputIndex);
                    if (inputBuffer != null) {
                        inputBuffer.clear();
                    }
                    int sampleSize = inputBuffer != null
                            ? extractor.readSampleData(inputBuffer, 0)
                            : -1;
                    if (sampleSize < 0) {
                        decoder.queueInputBuffer(
                                inputIndex,
                                0,
                                0,
                                0,
                                MediaCodec.BUFFER_FLAG_END_OF_STREAM
                        );
                        inputEnded = true;
                    } else {
                        decoder.queueInputBuffer(
                                inputIndex,
                                0,
                                sampleSize,
                                extractor.getSampleTime(),
                                extractor.getSampleFlags()
                        );
                        extractor.advance();
                    }
                }
            }

            int outputIndex = decoder.dequeueOutputBuffer(bufferInfo, CODEC_TIMEOUT_US);
            if (outputIndex >= 0) {
                if (playbackStartMs < 0) {
                    playbackStartMs = System.currentTimeMillis() - bufferInfo.presentationTimeUs / 1000L;
                }
                waitUntilPresentationTime(playbackStartMs, bufferInfo.presentationTimeUs);
                boolean render = (bufferInfo.flags & MediaCodec.BUFFER_FLAG_END_OF_STREAM) == 0;
                decoder.releaseOutputBuffer(outputIndex, render);
                if ((bufferInfo.flags & MediaCodec.BUFFER_FLAG_END_OF_STREAM) != 0) {
                    waitForPendingVideoFrames(imageHandler);
                    decoder.flush();
                    extractor.seekTo(0, MediaExtractor.SEEK_TO_PREVIOUS_SYNC);
                    inputEnded = false;
                    playbackStartMs = -1L;
                }
            } else if (outputIndex == MediaCodec.INFO_OUTPUT_FORMAT_CHANGED) {
                Log.d(LOG_TAG, "Video decoder output format changed: " + decoder.getOutputFormat());
            }
        }
    }

    private void waitForPendingVideoFrames(Handler imageHandler) {
        CountDownLatch imagesDrained = new CountDownLatch(1);
        CountDownLatch framesSubmitted = new CountDownLatch(1);
        imageHandler.post(() -> {
            imagesDrained.countDown();
            mainHandler.post(framesSubmitted::countDown);
        });
        try {
            imagesDrained.await();
            framesSubmitted.await();
        } catch (InterruptedException err) {
            Thread.currentThread().interrupt();
            cancelled = true;
        }
    }

    private void waitUntilPresentationTime(long playbackStartMs, long presentationTimeUs) {
        long delayMs = playbackStartMs + presentationTimeUs / 1000L - System.currentTimeMillis();
        if (delayMs <= 0) {
            return;
        }
        try {
            Thread.sleep(delayMs);
        } catch (InterruptedException err) {
            Thread.currentThread().interrupt();
            cancelled = true;
        }
    }

    private void submitHardwareFrame(
            int width,
            int height,
            int dataSpace,
            int rotationDegrees,
            HardwareBuffer hardwareBuffer,
            Runnable releaseOwner
    ) {
        postHardwareFrameWhenSurfaceIsReady(new HardwareFrame(
                width,
                height,
                dataSpace,
                rotationDegrees,
                hardwareBuffer,
                releaseOwner
        ));
    }

    private void postHardwareFrameWhenSurfaceIsReady(HardwareFrame frame) {
        boolean shouldPost = false;
        synchronized (hardwareFrameLock) {
            if (pendingHardwareFrame != null) {
                pendingHardwareFrame.close();
            }
            pendingHardwareFrame = frame;
            if (!hardwareFramePostScheduled) {
                hardwareFramePostScheduled = true;
                shouldPost = true;
            }
        }
        if (shouldPost) {
            mainHandler.post(this::postPendingHardwareFrameOnMainThread);
        }
    }

    private void postPendingHardwareFrameOnMainThread() {
        HardwareFrame frame;
        synchronized (hardwareFrameLock) {
            frame = pendingHardwareFrame;
            pendingHardwareFrame = null;
            if (frame == null) {
                hardwareFramePostScheduled = false;
                return;
            }
        }

        if (cancelled) {
            frame.close();
            synchronized (hardwareFrameLock) {
                hardwareFramePostScheduled = false;
            }
            return;
        }
        if (!simulatorView.getHolder().getSurface().isValid()) {
            mainHandler.postDelayed(
                    this::postPendingHardwareFrameOnMainThread,
                    SURFACE_RETRY_DELAY_MS
            );
            synchronized (hardwareFrameLock) {
                if (pendingHardwareFrame != null) {
                    frame.close();
                } else {
                    pendingHardwareFrame = frame;
                }
            }
            return;
        }
        try {
            simulatorView.postHardwareBuffer(
                    frame.width,
                    frame.height,
                    frame.dataSpace,
                    frame.rotationDegrees,
                    frame.hardwareBuffer
            );
        } finally {
            frame.close();
        }

        boolean shouldPostAgain;
        synchronized (hardwareFrameLock) {
            shouldPostAgain = pendingHardwareFrame != null;
            hardwareFramePostScheduled = shouldPostAgain;
        }
        if (shouldPostAgain) {
            mainHandler.post(this::postPendingHardwareFrameOnMainThread);
        }
    }

    private void closePendingHardwareFrame() {
        synchronized (hardwareFrameLock) {
            if (pendingHardwareFrame != null) {
                pendingHardwareFrame.close();
                pendingHardwareFrame = null;
            }
            hardwareFramePostScheduled = false;
        }
    }

    private void waitForAcquireFence(Image image) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) {
            return;
        }

        try (SyncFence fence = image.getFence()) {
            if (fence != null && fence.isValid() && !fence.awaitForever()) {
                Log.w(LOG_TAG, "Media acquire fence reported an error");
            }
        } catch (IOException e) {
            Log.w(LOG_TAG, "Cannot get media acquire fence; continuing without explicit wait", e);
        }
    }

    private static class HardwareFrame {
        final int width;
        final int height;
        final int dataSpace;
        final int rotationDegrees;
        final HardwareBuffer hardwareBuffer;
        final Runnable releaseOwner;

        HardwareFrame(
                int width,
                int height,
                int dataSpace,
                int rotationDegrees,
                HardwareBuffer hardwareBuffer,
                Runnable releaseOwner
        ) {
            this.width = width;
            this.height = height;
            this.dataSpace = dataSpace;
            this.rotationDegrees = rotationDegrees;
            this.hardwareBuffer = hardwareBuffer;
            this.releaseOwner = releaseOwner;
        }

        void close() {
            try {
                hardwareBuffer.close();
            } finally {
                releaseOwner.run();
            }
        }
    }

}
