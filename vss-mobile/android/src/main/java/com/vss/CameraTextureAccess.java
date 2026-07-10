package com.vss;

import static android.hardware.camera2.CameraDevice.TEMPLATE_PREVIEW;

import android.Manifest;
import android.content.Context;
import android.content.pm.PackageManager;
import android.graphics.ImageFormat;
import android.graphics.Point;
import android.graphics.Rect;
import android.hardware.HardwareBuffer;
import android.hardware.SyncFence;
import android.hardware.camera2.CameraAccessException;
import android.hardware.camera2.CameraCaptureSession;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraDevice;
import android.hardware.camera2.CameraManager;
import android.hardware.camera2.CaptureRequest;
import android.hardware.camera2.params.StreamConfigurationMap;
import android.media.Image;
import android.media.ImageReader;
import android.os.Build;
import android.util.Log;
import android.util.Range;
import android.util.Size;
import android.util.SizeF;
import android.view.Display;
import android.view.Surface;
import android.view.WindowManager;

import androidx.annotation.NonNull;
import androidx.annotation.RequiresApi;
import androidx.core.app.ActivityCompat;

import java.util.Arrays;
import java.util.List;
import java.io.IOException;

/**
 * Experimental camera path that passes GPU-backed HardwareBuffer frames to native code.
 */
@RequiresApi(api = Build.VERSION_CODES.P)
public class CameraTextureAccess {
    private final Context context;
    private final CameraDelegate delegate;

    private ImageReader imageReader;
    private CameraDevice cameraDevice;
    private CameraCaptureSession captureSession;

    public CameraTextureAccess(Context context, CameraDelegate delegate) {
        this.context = context;
        this.delegate = delegate;
        setupCamera();
    }

    private boolean checkCameraPermission() {
        if (ActivityCompat.checkSelfPermission(context, Manifest.permission.CAMERA) == PackageManager.PERMISSION_DENIED) {
            delegate.onCameraPermissionDenied();
            return false;
        }
        return true;
    }

    @NonNull
    private Size getScreenSize() {
        WindowManager windowManager =
                (WindowManager) context.getSystemService(Context.WINDOW_SERVICE);
        Display display = windowManager.getDefaultDisplay();
        Point size = new Point();
        display.getSize(size);
        return new Size(size.x, size.y);
    }

    private Size getBestResolution(List<Size> cameraSizes, Size screenSize) {
        final int minScreen = Math.min(screenSize.getWidth(), screenSize.getHeight());
        final int maxScreen = Math.max(screenSize.getWidth(), screenSize.getHeight());
        Size bestSize = null;
        int bestDiff = Integer.MAX_VALUE;
        for (Size size : cameraSizes) {
            final int diffA = Math.abs(Math.min(size.getWidth(), size.getHeight()) - minScreen);
            final int diffB = Math.abs(Math.max(size.getWidth(), size.getHeight()) - maxScreen);
            if (diffA < bestDiff || diffB < bestDiff) {
                bestDiff = Math.min(diffA, diffB);
                bestSize = size;
            }
        }
        return bestSize;
    }

    private CameraSelection selectCamera(CameraManager manager, int imageFormat, Size screenSize)
            throws CameraAccessException {
        CameraSelection best = null;
        for (String cameraId : manager.getCameraIdList()) {
            CameraCharacteristics characteristics = manager.getCameraCharacteristics(cameraId);
            Integer facing = characteristics.get(CameraCharacteristics.LENS_FACING);
            if (facing == null || facing != CameraCharacteristics.LENS_FACING_BACK) {
                continue;
            }

            StreamConfigurationMap map =
                    characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
            if (map == null || map.getOutputSizes(imageFormat) == null) {
                continue;
            }

            List<Size> outputSizes = Arrays.asList(map.getOutputSizes(imageFormat));
            if (outputSizes.isEmpty()) {
                Log.w("CameraTexture", "No YUV_420_888 output sizes for camera " + cameraId);
                continue;
            }

            Size bestSize = getBestResolution(outputSizes, screenSize);
            float normalFovError = normalCameraFovError(characteristics);
            boolean logicalCamera = hasCapability(
                    characteristics,
                    CameraCharacteristics.REQUEST_AVAILABLE_CAPABILITIES_LOGICAL_MULTI_CAMERA
            );
            boolean zoomRatioSupportsOne = zoomRatioSupportsOne(characteristics);
            int score = 1000;
            if (logicalCamera) {
                score += 500;
            }
            if (zoomRatioSupportsOne) {
                score += 300;
            }
            if (normalFovError < Float.MAX_VALUE) {
                score += Math.max(0, Math.round(300 - normalFovError * 10));
            }

            Log.i("CameraTexture",
                    "Camera candidate " + cameraId
                            + ": score=" + score
                            + ", size=" + bestSize
                            + ", logical=" + logicalCamera
                            + ", zoom1=" + zoomRatioSupportsOne
                            + ", normalFovError=" + normalFovError);

            CameraSelection candidate = new CameraSelection(
                    cameraId,
                    characteristics,
                    bestSize,
                    sensorOrientation(characteristics),
                    zoomRatioSupportsOne,
                    score
            );
            if (best == null || candidate.score > best.score) {
                best = candidate;
            }
        }
        return best;
    }

    private boolean hasCapability(CameraCharacteristics characteristics, int capability) {
        int[] capabilities =
                characteristics.get(CameraCharacteristics.REQUEST_AVAILABLE_CAPABILITIES);
        if (capabilities == null) {
            return false;
        }
        for (int candidate : capabilities) {
            if (candidate == capability) {
                return true;
            }
        }
        return false;
    }

    private boolean zoomRatioSupportsOne(CameraCharacteristics characteristics) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) {
            return false;
        }

        Range<Float> zoomRange = characteristics.get(CameraCharacteristics.CONTROL_ZOOM_RATIO_RANGE);
        return zoomRange != null && zoomRange.contains(1.0f);
    }

    private float normalCameraFovError(CameraCharacteristics characteristics) {
        SizeF sensorSize = characteristics.get(CameraCharacteristics.SENSOR_INFO_PHYSICAL_SIZE);
        float[] focalLengths = characteristics.get(CameraCharacteristics.LENS_INFO_AVAILABLE_FOCAL_LENGTHS);
        if (sensorSize == null || focalLengths == null || focalLengths.length == 0) {
            return Float.MAX_VALUE;
        }

        final float targetHorizontalFovDegrees = 75.0f;
        float bestError = Float.MAX_VALUE;
        for (float focalLength : focalLengths) {
            if (focalLength <= 0.0f) {
                continue;
            }
            float fov = (float) Math.toDegrees(
                    2.0 * Math.atan(sensorSize.getWidth() / (2.0 * focalLength))
            );
            bestError = Math.min(bestError, Math.abs(fov - targetHorizontalFovDegrees));
        }
        return bestError;
    }

    private int sensorOrientation(CameraCharacteristics characteristics) {
        Integer orientation = characteristics.get(CameraCharacteristics.SENSOR_ORIENTATION);
        return orientation != null ? orientation : 0;
    }

    private int displayRotationDegrees() {
        WindowManager windowManager =
                (WindowManager) context.getSystemService(Context.WINDOW_SERVICE);
        Display display = windowManager.getDefaultDisplay();
        switch (display.getRotation()) {
            case Surface.ROTATION_90:
                return 90;
            case Surface.ROTATION_180:
                return 180;
            case Surface.ROTATION_270:
                return 270;
            case Surface.ROTATION_0:
            default:
                return 0;
        }
    }

    private void setupCamera() {
        if (!checkCameraPermission()) {
            return;
        }

        CameraManager manager = (CameraManager) context.getSystemService(Context.CAMERA_SERVICE);
        try {
            final int imageFormat = ImageFormat.YUV_420_888;
            Size screenSize = getScreenSize();
            CameraSelection selection = selectCamera(manager, imageFormat, screenSize);
            if (selection == null) {
                Log.e("CameraTexture", "No suitable back-facing YUV_420_888 camera found");
                return;
            }

            Log.i("CameraTexture",
                    "Using hardware-buffer camera " + selection.cameraId
                            + " at " + selection.size
                            + " (sensor orientation is " + selection.sensorOrientationDegrees
                            + ", display rotation is " + displayRotationDegrees() + ")"
                            + " (screen resolution is " + screenSize + ")");

            manager.openCamera(selection.cameraId, new CameraDevice.StateCallback() {
                @Override
                public void onOpened(@NonNull CameraDevice cameraDevice) {
                    Log.i("CameraTexture", "Device opened");
                    delegate.onCameraOpen(cameraDevice);
                    setupCameraSession(cameraDevice, imageFormat, selection);
                }

                @Override
                public void onDisconnected(@NonNull CameraDevice cameraDevice) {
                    Log.i("CameraTexture", "Device disconnected");
                    delegate.onCameraDisconnected(cameraDevice);
                }

                @Override
                public void onError(@NonNull CameraDevice cameraDevice, int error) {
                    Log.e("CameraTexture", "Device error (" + error + ")");
                    delegate.onCameraError(cameraDevice, error);
                }
            }, null);
        } catch (CameraAccessException e) {
            Log.e("CameraTexture", "Setting up camera failed", e);
        }
    }

    private void setupCameraSession(CameraDevice cameraDevice, int imageFormat, CameraSelection selection) {
        Size size = selection.size;
        final int width = size.getWidth();
        final int height = size.getHeight();

        imageReader = ImageReader.newInstance(width, height, imageFormat, 2);
        imageReader.setOnImageAvailableListener(reader -> {
            Image image = reader.acquireLatestImage();
            if (image == null) {
                return;
            }

            HardwareBuffer hardwareBuffer = null;
            try {
                waitForAcquireFence(image);
                hardwareBuffer = image.getHardwareBuffer();
                if (hardwareBuffer != null) {
                    delegate.onFrameAvailable(
                            width,
                            height,
                            image.getDataSpace(),
                            selection.outputRotationDegrees(displayRotationDegrees()),
                            hardwareBuffer
                    );
                }
            } finally {
                if (hardwareBuffer != null) {
                    hardwareBuffer.close();
                }
                image.close();
            }
        }, null);

        try {
            cameraDevice.createCaptureSession(Arrays.asList(imageReader.getSurface()), new CameraCaptureSession.StateCallback() {
                @Override
                public void onConfigured(@NonNull CameraCaptureSession session) {
                    captureSession = session;
                    try {
                        CaptureRequest.Builder requestBuilder =
                                cameraDevice.createCaptureRequest(TEMPLATE_PREVIEW);
                        requestBuilder.addTarget(imageReader.getSurface());
                        requestBuilder.set(CaptureRequest.CONTROL_AF_MODE,
                                CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_PICTURE);
                        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.R && selection.zoomRatioSupportsOne) {
                            requestBuilder.set(CaptureRequest.CONTROL_ZOOM_RATIO, 1.0f);
                        } else {
                            Rect activeArray = selection.characteristics.get(
                                    CameraCharacteristics.SENSOR_INFO_ACTIVE_ARRAY_SIZE
                            );
                            if (activeArray != null) {
                                requestBuilder.set(CaptureRequest.SCALER_CROP_REGION, activeArray);
                            }
                        }
                        captureSession.setRepeatingRequest(requestBuilder.build(),
                                new CameraCaptureSession.CaptureCallback() {
                                }, null);
                    } catch (CameraAccessException e) {
                        Log.e("CameraTextureSession", "Configure failed", e);
                    }
                }

                @Override
                public void onConfigureFailed(@NonNull CameraCaptureSession session) {
                    Log.e("CameraTextureSession", "Configure failed");
                }
            }, null);
        } catch (CameraAccessException e) {
            Log.e("CameraTextureSession", "Creating session", e);
        }

        this.cameraDevice = cameraDevice;
    }

    private static class CameraSelection {
        final String cameraId;
        final CameraCharacteristics characteristics;
        final Size size;
        final int sensorOrientationDegrees;
        final boolean zoomRatioSupportsOne;
        final int score;

        CameraSelection(
                String cameraId,
                CameraCharacteristics characteristics,
                Size size,
                int sensorOrientationDegrees,
                boolean zoomRatioSupportsOne,
                int score
        ) {
            this.cameraId = cameraId;
            this.characteristics = characteristics;
            this.size = size;
            this.sensorOrientationDegrees = sensorOrientationDegrees;
            this.zoomRatioSupportsOne = zoomRatioSupportsOne;
            this.score = score;
        }

        int outputRotationDegrees(int displayRotationDegrees) {
            return ((sensorOrientationDegrees - displayRotationDegrees) % 360 + 360) % 360;
        }
    }

    private void waitForAcquireFence(Image image) {
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU) {
            return;
        }

        try (SyncFence fence = image.getFence()) {
            if (fence != null && fence.isValid() && !fence.awaitForever()) {
                Log.w("CameraTexture", "Camera acquire fence reported an error");
            }
        } catch (IOException e) {
            Log.w("CameraTexture", "Cannot get camera acquire fence; continuing without explicit wait", e);
        }
    }

    void close() {
        if (captureSession != null) {
            captureSession.close();
            captureSession = null;
        }
        if (cameraDevice != null) {
            cameraDevice.close();
            cameraDevice = null;
        }
        if (imageReader != null) {
            imageReader.close();
            imageReader = null;
        }
    }

    public interface CameraDelegate {
        void onCameraOpen(CameraDevice cameraDevice);

        void onCameraDisconnected(CameraDevice cameraDevice);

        void onCameraError(CameraDevice cameraDevice, int error);

        void onCameraPermissionDenied();

        void onFrameAvailable(
                int width,
                int height,
                int dataSpace,
                int rotationDegrees,
                HardwareBuffer hardwareBuffer
        );
    }
}
