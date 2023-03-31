package com.vss;

import static android.hardware.camera2.CameraDevice.TEMPLATE_PREVIEW;

import android.Manifest;
import android.content.Context;
import android.content.pm.PackageManager;
import android.graphics.ImageFormat;
import android.graphics.Point;
import android.hardware.camera2.CameraAccessException;
import android.hardware.camera2.CameraCaptureSession;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraDevice;
import android.hardware.camera2.CameraManager;
import android.hardware.camera2.CaptureRequest;
import android.hardware.camera2.params.StreamConfigurationMap;
import android.media.Image;
import android.media.ImageReader;
import android.util.Log;
import android.util.Size;
import android.view.Display;
import android.view.WindowManager;

import androidx.annotation.NonNull;
import androidx.core.app.ActivityCompat;

import java.nio.BufferUnderflowException;
import java.util.Arrays;
import java.util.List;

/**
 * Camera access helper.
 */
public class CameraAccess {

    private static final int CAMERA_REQUEST_CODE = 100;

    private final Context context;
    private final CameraDelegate delegate;

    private ImageReader imageReader;
    private CameraDevice cameraDevice;

    public CameraAccess(Context context, CameraDelegate delegate) {
        this.context = context;
        this.delegate = delegate;
        setupCamera();
    }

    private boolean checkCameraPermission() {
        if (ActivityCompat.checkSelfPermission(context, Manifest.permission.CAMERA) == PackageManager.PERMISSION_DENIED) {
            delegate.onCameraPermissionDenied();
            return false;
        } else {
            return true;
        }
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

    /**
     * This function selects the camera resolution that matches the screen size best.
     *
     * @param cameraSizes possible camera resolutions
     * @param screenSize  the screen size.
     * @return best matching camera resolution.
     */
    private Size getBestResolution(List<Size> cameraSizes, Size screenSize) {
        final int minScreen = Math.min(screenSize.getWidth(), screenSize.getHeight());
        final int maxScreen = Math.max(screenSize.getWidth(), screenSize.getHeight());
        Size bestSize = null;
        int bestDiff = Integer.MAX_VALUE;
        for (Size size : cameraSizes) {
            final int diffA = Math.abs(Math.min(size.getWidth(), size.getHeight()) - minScreen);
            final int diffB = Math.abs(Math.max(size.getWidth(), size.getHeight()) - maxScreen);
            if ((diffA < bestDiff || diffB < bestDiff)) {
                bestDiff = Math.min(diffA, diffB);
                bestSize = size;
            }
        }
        return bestSize;
    }

    private void setupCamera() {
        if (!checkCameraPermission()) {
            return;
        }

        CameraManager manager = (CameraManager) context.getSystemService(Context.CAMERA_SERVICE);
        try {
            for (String cameraId : manager.getCameraIdList()) {
                CameraCharacteristics characteristics = manager.getCameraCharacteristics(cameraId);

                // Do not use front facing camera.
                Integer facing = characteristics.get(CameraCharacteristics.LENS_FACING);
                if (facing != null && facing == CameraCharacteristics.LENS_FACING_FRONT) {
                    continue;
                }

                // We need a stream configuration.
                StreamConfigurationMap map =
                        characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
                if (map == null) {
                    continue;
                }

                // Determine possible output sizes.
                final int imageFormat = ImageFormat.YUV_420_888;
                final List<Size> outputSizes = Arrays.asList(map.getOutputSizes(imageFormat));

                // Log useful information and selection.
                Log.i("Camera", map.toString());
                for (Size size : outputSizes) {
                    Log.i("Camera", size.toString());
                }


                // Select camera resolution.
                Size screenSize = getScreenSize();
                final Size bestSize = getBestResolution(outputSizes, screenSize);
                Log.i("Camera",
                        "Using camera resolution " + bestSize + " (screen resolution is " + screenSize + ")");

                //TODO: use multi-camera API (camera2) or FEATURE_CAMERA_CONCURRENT (camera2) for
                // front/back camera

                manager.openCamera(cameraId, new CameraDevice.StateCallback() {
                    @Override
                    public void onOpened(CameraDevice cameraDevice) {
                        Log.i("Camera", "Device opened");
                        delegate.onCameraOpen(cameraDevice);
                        setupCameraSession(cameraDevice, imageFormat, bestSize);
                    }

                    @Override
                    public void onDisconnected(CameraDevice cameraDevice) {
                        Log.i("Camera", "Device disconnected");
                        delegate.onCameraDisconnected(cameraDevice);
                    }

                    @Override
                    public void onError(CameraDevice cameraDevice, int error) {
                        Log.e("Camera", "Device error (" + error + ")");
                        delegate.onCameraError(cameraDevice, error);
                    }
                }, null);

                // Bail out if we found a suitable one.
                break;
            }
        } catch (CameraAccessException e) {
            Log.e("Camera", "Setting up camera failed", e);
        }
    }

    private void setupCameraSession(CameraDevice cameraDevice, int imageFormat, Size largestSize) {
        final int width = largestSize.getWidth();
        final int height = largestSize.getHeight();

        // Create buffers for copying channels to.
        int size = width * height;
        byte[] y = new byte[size];
        byte[] u = new byte[size / 2 - 1];
        byte[] v = new byte[size / 2 - 1];

        // Create image reader for accessing pixel data.
        imageReader = ImageReader.newInstance(width, height, imageFormat, 2);
        imageReader.setOnImageAvailableListener(new ImageReader.OnImageAvailableListener() {
            private boolean logUnexpected = true;

            public void onImageAvailable(ImageReader reader) {
                final Image image = reader.acquireLatestImage();
                if (image == null) {
                    return;
                }

                try {
                    // Copy planes to buffers.
                    image.getPlanes()[0].getBuffer().get(y);
                    image.getPlanes()[1].getBuffer().get(u);
                    image.getPlanes()[2].getBuffer().get(v);

                    // Emit frame.
                    delegate.onFrameAvailable(width, height, y, u, v);
                } catch (BufferUnderflowException e) {
                    if (logUnexpected) {
                        Log.w("ImageReader", "Unexpectedly sized frame. Ignoring.");
                        logUnexpected = false;
                    }
                }

                // Free manually
                image.close();
            }
        }, null);

        try {
            // Create a CameraCaptureSession for camera preview.
            final List outputs = Arrays.asList(imageReader.getSurface());
            cameraDevice.createCaptureSession(outputs, new CameraCaptureSession.StateCallback() {
                @Override
                public void onConfigured(@NonNull CameraCaptureSession captureSession) {
                    try {
                        CaptureRequest.Builder requestBuilder =
                                cameraDevice.createCaptureRequest(TEMPLATE_PREVIEW);
                        requestBuilder.addTarget(imageReader.getSurface());

                        // Set continuous auto focus.
                        requestBuilder.set(CaptureRequest.CONTROL_AF_MODE,
                                CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_PICTURE);

                        // Finally, start requesting frames.
                        captureSession.setRepeatingRequest(requestBuilder.build(),
                                new CameraCaptureSession.CaptureCallback() {
                        }, null);
                    } catch (CameraAccessException e) {
                        Log.e("CameraSession", "Configure failed", e);
                    }
                }

                @Override
                public void onConfigureFailed(@NonNull CameraCaptureSession cameraCaptureSession) {
                    Log.e("CameraSession", "Configure failed");
                }
            }, null);
        } catch (CameraAccessException e) {
            Log.e("CameraSession", "Creating session", e);
        }

        this.cameraDevice = cameraDevice;
    }

    void close() {
        if (cameraDevice != null) {
            cameraDevice.close();
        }
    }

    public interface CameraDelegate {
        void onCameraOpen(CameraDevice cameraDevice);

        void onCameraDisconnected(CameraDevice cameraDevice);

        void onCameraError(CameraDevice cameraDevice, int error);

        void onCameraPermissionDenied();

        void onFrameAvailable(int width, int height, byte[] y, byte[] u, byte[] v);
    }

}
