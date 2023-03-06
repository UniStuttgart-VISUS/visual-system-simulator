package com.vss;

import static android.hardware.camera2.CameraDevice.TEMPLATE_PREVIEW;

import android.Manifest;
import android.content.Context;
import android.content.pm.PackageManager;
import android.graphics.ImageFormat;
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

import androidx.annotation.NonNull;
import androidx.core.app.ActivityCompat;

import java.util.Arrays;
import java.util.Collections;
import java.util.Comparator;
import java.util.List;

/**
 * Camera access helper.
 */
public class CameraAccess {

    private static final int CAMERA_REQUEST_CODE = 100;

    private final Context context;
    private final CameraDelegate delegate;

    private ImageReader imageReader;

    public CameraAccess(Context context, CameraDelegate delegate) {
        this.context = context;
        this.delegate = delegate;
        setupCamera();
    }

    private boolean checkCameraPermission() {
        if (ActivityCompat.checkSelfPermission(context,
                Manifest.permission.CAMERA) == PackageManager.PERMISSION_DENIED) {
            delegate.onCameraPermissionDenied();
            return false;
        } else {
            return true;
        }
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
                StreamConfigurationMap map = characteristics.get(CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
                if (map == null) {
                    continue;
                }

                // Determine possible output sizes.
                final int imageFormat = ImageFormat.YUV_420_888;
                final List<Size> outputSizes = Arrays.asList(map.getOutputSizes(imageFormat));

                // Log some useful information.
                Log.i("Camera", map.toString());
                for (Size size : outputSizes) {
                    Log.i("Camera", size.toString());
                }

                // Use the largest available size for the given format.
                final Size largestSize = Collections.max(outputSizes, new Comparator<Size>() {
                    @Override
                    public int compare(Size lhs, Size rhs) {
                        // We cast here to ensure the multiplications won't overflow
                        return Long.signum((long) lhs.getWidth() * lhs.getHeight() - (long) rhs.getWidth() * rhs.getHeight());
                    }
                });

                //TODO: use multi-camera API (camera2) or FEATURE_CAMERA_CONCURRENT (camera2) for front/back camera

                manager.openCamera(cameraId, new CameraDevice.StateCallback() {
                    @Override
                    public void onOpened(CameraDevice cameraDevice) {
                        Log.i("Camera", "Device opened");
                        delegate.onCameraOpen(cameraDevice);
                        setupCameraSession(cameraDevice, imageFormat, largestSize);
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
            public void onImageAvailable(ImageReader reader) {
                Image image = reader.acquireLatestImage();
                if (image == null) {
                    return;
                }

                // Copy planes to buffers.
                image.getPlanes()[0].getBuffer().get(y);
                image.getPlanes()[1].getBuffer().get(u);
                image.getPlanes()[2].getBuffer().get(v);

                // Emit frame.
                delegate.onFrameAvailable(width, height, y, u, v);

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
                        CaptureRequest.Builder requestBuilder = cameraDevice.createCaptureRequest(TEMPLATE_PREVIEW);
                        requestBuilder.addTarget(imageReader.getSurface());

                        // Set continuous auto focus.
                        requestBuilder.set(CaptureRequest.CONTROL_AF_MODE, CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_PICTURE);

                        // Finally, start requesting frames.
                        captureSession.setRepeatingRequest(requestBuilder.build(), new CameraCaptureSession.CaptureCallback() {
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
    }

    public interface CameraDelegate {
        void onCameraOpen(CameraDevice cameraDevice);

        void onCameraDisconnected(CameraDevice cameraDevice);

        void onCameraError(CameraDevice cameraDevice, int error);

        void onCameraPermissionDenied();

        void onFrameAvailable(int width, int height, byte[] y, byte[] u, byte[] v);
    }

}
