package de.uni_stuttgart.vss.activities;

/* this class contains code from The Android Open Source Project,
 Licensed under the Apache License, Version 2.0
 The original can be found here: https://github.com/googlesamples/android-Camera2Basic/blob/master/
 Application/src/main/java/com/example/android/camera2basic/Camera2BasicFragment.java
*/

import android.Manifest;
import android.app.Activity;
import android.app.NativeActivity;
import android.content.Context;
import android.content.pm.PackageManager;
import android.graphics.ImageFormat;
import android.graphics.Matrix;
import android.graphics.Point;
import android.graphics.RectF;
import android.graphics.SurfaceTexture;
import android.hardware.camera2.CameraAccessException;
import android.hardware.camera2.CameraCaptureSession;
import android.hardware.camera2.CameraCharacteristics;
import android.hardware.camera2.CameraDevice;
import android.hardware.camera2.CameraManager;
import android.hardware.camera2.CaptureRequest;
import android.hardware.camera2.CaptureResult;
import android.hardware.camera2.TotalCaptureResult;
import android.hardware.camera2.params.StreamConfigurationMap;
import android.media.Image;
import android.media.ImageReader;
import android.opengl.GLES20;
import android.os.Bundle;
import android.os.Handler;
import android.support.annotation.NonNull;
import android.support.v4.app.ActivityCompat;
import android.util.Log;
import android.util.Size;
import android.view.Surface;
import android.view.WindowManager;

import java.util.ArrayList;
import java.util.Arrays;
import java.util.Collections;
import java.util.Comparator;
import java.util.List;
import java.util.Map;
import java.util.concurrent.Semaphore;
import java.util.concurrent.TimeUnit;

import de.uni_stuttgart.vss.edsettings.EDSettingsController;
import de.uni_stuttgart.vss.edsettings.EDSettingsListener;

import static android.opengl.GLES11Ext.GL_TEXTURE_EXTERNAL_OES;

/**
 * Created by marco on 9/16/17.
 */

public class CoreActivity extends NativeActivity implements EDSettingsListener {

    static {

    }

    private boolean active;

    private int[] _myTextureId;
    private SurfaceTexture _mySurfaceTexture;
    private CameraDevice mCameraDevice;
    private Semaphore mCameraOpenCloseLock = new Semaphore(1);
    private CaptureRequest.Builder mPreviewRequestBuilder;
    private CameraCaptureSession mCaptureSession;
    private Size mPreviewSize;
    private CaptureRequest mPreviewRequest;
    private ImageReader mImageReader;
    private Handler mBackgroundHandler;
    private String mCameraId;
    private int mSensorOrientation;
    /**
     * Max preview width that is guaranteed by Camera2 API
     */
    private static final int MAX_PREVIEW_WIDTH = 1920;

    /**
     * Max preview height that is guaranteed by Camera2 API
     */
    private static final int MAX_PREVIEW_HEIGHT = 1080;

    private static long accTotal = 0;
    private static long accAlloc = 0;
    private static long accCopy = 0;
    private static long accPost = 0;
    private static long timeCounter = 0;
    private static long ACCCOUNT = 30;

    private static int rot = 90;
    private static int width = 1080;
    private static int height = 1920;




/*
START Android Lifecycle
*/

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        System.out.println("#-# OnCreate");
        

        
        /*edIf.changeValues("xres", Integer.toString(width));
        edIf.changeValues("yres", Integer.toString(height));*/
        
        String simSettings = EDSettingsController.getSimulationSettings();
        simSettings = new StringBuilder(simSettings).insert(simSettings.length() - 1, String.format(",\"xres\":%d,\"yres\":%d", width, height)).toString();
        System.out.println("#-# Settings: "+ simSettings);
        postConfig(simSettings);
        System.out.println("#-# OnCreateAfterPostConfig");

        EDSettingsListener.Provider.getInstance().addListener(this);

        //we do not want to simulate darkness
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);


        int[] textures = new int[1];
        GLES20.glGenTextures(1, textures, 0);

        _myTextureId = textures;
        GLES20.glBindTexture(GL_TEXTURE_EXTERNAL_OES, _myTextureId[0]);

        GLES20.glTexParameterf(GL_TEXTURE_EXTERNAL_OES, GLES20.GL_TEXTURE_MIN_FILTER,
                GLES20.GL_NEAREST);
        GLES20.glTexParameterf(GL_TEXTURE_EXTERNAL_OES, GLES20.GL_TEXTURE_MAG_FILTER,
                GLES20.GL_LINEAR);

        _mySurfaceTexture = new SurfaceTexture(_myTextureId[0]);
        _mySurfaceTexture.setOnFrameAvailableListener(new SurfaceTexture.OnFrameAvailableListener() {
            @Override
            public void onFrameAvailable(SurfaceTexture surfaceTexture) {
                //Log.i("###", "onFrameAvailable called");
                //surfaceTexture.
                //CoreLoaderActivity.getFoo();
            }
        });

        //Surface mySurface = new Surface(_mySurfaceTexture);


        //int width = 640; NACH OBEN VERSCHOBEN
        //int height = 480;
        setUpCameraOutputs(width, height);
        configureTransform(width, height);
        Activity activity = this;
        CameraManager manager = (CameraManager) activity.getSystemService(Context.CAMERA_SERVICE);
        try {
            if (!mCameraOpenCloseLock.tryAcquire(2500, TimeUnit.MILLISECONDS)) {
                throw new RuntimeException("Time out waiting to lock camera opening.");
            }
            if (ActivityCompat.checkSelfPermission(this, Manifest.permission.CAMERA) != PackageManager.PERMISSION_GRANTED) {
                Log.e("###", "perm.problem");
                return;
            }
            manager.openCamera(mCameraId, mStateCallback, mBackgroundHandler);
        } catch (CameraAccessException e) {
            e.printStackTrace();
        } catch (InterruptedException e) {
            throw new RuntimeException("Interrupted while trying to lock camera opening.", e);
        }

    }

    @Override
    protected void onResume() {
        super.onResume();
        this.active = true;
    }

    @Override
    public void onPause() {
        try {
            mCaptureSession.stopRepeating();
            mCaptureSession.abortCaptures();
            try {
                mCameraOpenCloseLock.acquire();
                if (null != mCaptureSession) {
                    mCaptureSession.close();
                    mCaptureSession = null;
                }
                if (null != mCameraDevice) {
                    mCameraDevice.close();
                    mCameraDevice = null;
                }
                if (null != mImageReader) {
                    mImageReader.close();
                    mImageReader = null;
                }
            } catch (InterruptedException e) {
                throw new RuntimeException("Interrupted while trying to lock camera closing.", e);
            } finally {
                mCameraOpenCloseLock.release();
            }
        } catch (CameraAccessException e) {
            e.printStackTrace();
        }
        this.finish();
        super.onPause();
        this.active = false;
    }

/*
END Android Lifecycle
*/



/*
START Camera
*/

    public static native void postData(int width, int height, byte[] y, byte[] u, byte[] v);

    private final ImageReader.OnImageAvailableListener mOnImageAvailableListener
            = new ImageReader.OnImageAvailableListener() {

        @Override
        public void onImageAvailable(ImageReader reader) {
            long onEnter = System.nanoTime();
            Image img = reader.acquireLatestImage();
            if (img == null) {
               return;
            }

            long onAlloc = System.nanoTime();

            int size = 1920*1080;
            byte[] y = new byte[size];
            byte[] u = new byte[size / 2 - 1];
            byte[] v = new byte[size / 2 - 1];

            long onCopy = System.nanoTime();

            img.getPlanes()[0].getBuffer().get(y);
            img.getPlanes()[1].getBuffer().get(u);
            img.getPlanes()[2].getBuffer().get(v);




            /*
            Log.i("++++","nunmplanes: "+img.getPlanes().length);
            Log.i("++++","planelen0: "+img.getPlanes()[0].getBuffer().capacity());
            Log.i("++++","planelen1: "+img.getPlanes()[1].getBuffer().capacity());
            Log.i("++++","planelen2: "+img.getPlanes()[2].getBuffer().capacity());
            Log.i("++++","planepixelstride0: "+img.getPlanes()[0].getPixelStride());
            Log.i("++++","planerowstride0: "+img.getPlanes()[0].getRowStride());
            Log.i("++++","planepixelstride1: "+img.getPlanes()[1].getPixelStride());
            Log.i("++++","planerowstride1: "+img.getPlanes()[1].getRowStride());
            Log.i("++++","planepixelstride2: "+img.getPlanes()[2].getPixelStride());
            Log.i("++++","planerowstride2: "+img.getPlanes()[2].getRowStride());
            */


            long onPost = System.nanoTime();

            postData(0, 0, y, u, v);

            long onEnd = System.nanoTime();

            accTotal += (onEnd-onEnter)/1000;
            accAlloc += (onCopy-onAlloc)/1000;
            accCopy += (onPost-onCopy)/1000;
            accPost += (onEnd-onPost)/1000;
            timeCounter = (timeCounter+1) % ACCCOUNT;


            img.close();
        }
    };

    private final CameraDevice.StateCallback mStateCallback = new CameraDevice.StateCallback() {

        @Override
        public void onOpened(CameraDevice cameraDevice) {
            // This method is called when the camera is opened.  We start camera preview here.
            mCameraOpenCloseLock.release();
            mCameraDevice = cameraDevice;
            createCameraPreviewSession();
        }

        @Override
        public void onDisconnected(CameraDevice cameraDevice) {
            mCameraOpenCloseLock.release();
            cameraDevice.close();
            mCameraDevice = null;
        }

        @Override
        public void onError(CameraDevice cameraDevice, int error) {
            mCameraOpenCloseLock.release();
            cameraDevice.close();
            mCameraDevice = null;
            Log.e("###", "Could not open cam!");
        }

    };

    private CameraCaptureSession.CaptureCallback mCaptureCallback
            = new CameraCaptureSession.CaptureCallback() {

        @Override
        public void onCaptureProgressed(@NonNull CameraCaptureSession session,
                                        @NonNull CaptureRequest request,
                                        @NonNull CaptureResult partialResult) {
            //Log.i("###","onCaptureProgressed called");

        }

        @Override
        public void onCaptureCompleted(@NonNull CameraCaptureSession session,
                                       @NonNull CaptureRequest request,
                                       @NonNull TotalCaptureResult result) {
            //Log.i("###","onCaptureCompleted called");
        }

    };

    private void createCameraPreviewSession() {
        try {


            // We configure the size of default buffer to be the size of camera preview we want.
            _mySurfaceTexture.setDefaultBufferSize(mPreviewSize.getWidth(), mPreviewSize.getHeight());

            // We set up a CaptureRequest.Builder with the output Surface.
            mPreviewRequestBuilder
                    = mCameraDevice.createCaptureRequest(CameraDevice.TEMPLATE_PREVIEW);
            mPreviewRequestBuilder.addTarget(mImageReader.getSurface());

            Log.i("###", "camera opened");

            // Here, we create a CameraCaptureSession for camera preview.
            mCameraDevice.createCaptureSession(Arrays.asList(mImageReader.getSurface()),
                    new CameraCaptureSession.StateCallback() {

                        @Override
                        public void onConfigured(@NonNull CameraCaptureSession cameraCaptureSession) {
                            // The camera is already closed
                            if (null == mCameraDevice) {
                                Log.e("###", "mCameraDevice was null");

                                return;
                            }

                            // When the session is ready, we start displaying the preview.
                            mCaptureSession = cameraCaptureSession;
                            try {
                                // Auto focus should be continuous for camera preview.
                                mPreviewRequestBuilder.set(CaptureRequest.CONTROL_AF_MODE,
                                        CaptureRequest.CONTROL_AF_MODE_CONTINUOUS_PICTURE);

                                // Finally, we start displaying the camera preview.
                                mPreviewRequest = mPreviewRequestBuilder.build();
                                mCaptureSession.setRepeatingRequest(mPreviewRequest,
                                        mCaptureCallback, mBackgroundHandler);
                            } catch (CameraAccessException e) {
                                e.printStackTrace();
                            }
                        }

                        @Override
                        public void onConfigureFailed(
                                @NonNull CameraCaptureSession cameraCaptureSession) {
                            Log.e("###", "onConfigureFailed called");
                        }
                    }, null
            );
        } catch (CameraAccessException e) {
            e.printStackTrace();
        }
    }

    private void setUpCameraOutputs(int width, int height) {
        Activity activity = this;
        CameraManager manager = (CameraManager) activity.getSystemService(Context.CAMERA_SERVICE);
        try {
            for (String cameraId : manager.getCameraIdList()) {
                CameraCharacteristics characteristics
                        = manager.getCameraCharacteristics(cameraId);

                // We don't use a front facing camera in this sample.
                Integer facing = characteristics.get(CameraCharacteristics.LENS_FACING);
                if (facing != null && facing == CameraCharacteristics.LENS_FACING_FRONT) {
                    continue;
                }

                StreamConfigurationMap map = characteristics.get(
                        CameraCharacteristics.SCALER_STREAM_CONFIGURATION_MAP);
                if (map == null) {
                    continue;
                }

                Log.i("CameraFormat",map.toString());

                List<Size> l = Arrays.asList(map.getOutputSizes(ImageFormat.YUV_420_888));
                for (Size s : l) {
                    Log.i("TAG", s.toString());
                }
                // For still image captures, we use the largest available size.
                //Size largest = Collections.max(
                //        Arrays.asList(map.getOutputSizes(ImageFormat.YUV_420_888)),
                //        new CompareSizesByArea());

                Size largest = new Size(1920, 1080);
                mImageReader = ImageReader.newInstance(largest.getWidth(), largest.getHeight(),
                        ImageFormat.YUV_420_888, /*maxImages*/2);
                mImageReader.setOnImageAvailableListener(
                        mOnImageAvailableListener, mBackgroundHandler);

                // Find out if we need to swap dimension to get the preview size relative to sensor
                // coordinate.
                int displayRotation = activity.getWindowManager().getDefaultDisplay().getRotation();
                //noinspection ConstantConditions
                mSensorOrientation = characteristics.get(CameraCharacteristics.SENSOR_ORIENTATION);

                String simSettings = EDSettingsController.getSimulationSettings();
                simSettings = new StringBuilder(simSettings).insert(simSettings.length() - 1, String.format(",\"xres\":%d,\"yres\":%d,\"rotation\":%d",width, height,mSensorOrientation)).toString();
                postConfig(simSettings);

                boolean swappedDimensions = false;
                switch (displayRotation) {
                    case Surface.ROTATION_0:
                    case Surface.ROTATION_180:
                        if (mSensorOrientation == 90 || mSensorOrientation == 270) {
                            swappedDimensions = true;
                        }
                        break;
                    case Surface.ROTATION_90:
                    case Surface.ROTATION_270:
                        if (mSensorOrientation == 0 || mSensorOrientation == 180) {
                            swappedDimensions = true;
                        }
                        break;
                    default:
                        Log.e("xxx", "Display rotation is invalid: " + displayRotation);
                }

                Point displaySize = new Point();
                activity.getWindowManager().getDefaultDisplay().getSize(displaySize);
                int rotatedPreviewWidth = width;
                int rotatedPreviewHeight = height;
                int maxPreviewWidth = displaySize.x;
                int maxPreviewHeight = displaySize.y;

                if (swappedDimensions) {
                    rotatedPreviewWidth = height;
                    rotatedPreviewHeight = width;
                    maxPreviewWidth = displaySize.y;
                    maxPreviewHeight = displaySize.x;
                }

                if (maxPreviewWidth > MAX_PREVIEW_WIDTH) {
                    maxPreviewWidth = MAX_PREVIEW_WIDTH;
                }

                if (maxPreviewHeight > MAX_PREVIEW_HEIGHT) {
                    maxPreviewHeight = MAX_PREVIEW_HEIGHT;
                }

                // Danger, W.R.! Attempting to use too large a preview size could  exceed the camera
                // bus' bandwidth limitation, resulting in gorgeous previews but the storage of
                // garbage capture data.
                mPreviewSize = chooseOptimalSize(map.getOutputSizes(SurfaceTexture.class),
                        rotatedPreviewWidth, rotatedPreviewHeight, maxPreviewWidth,
                        maxPreviewHeight, largest);


                mCameraId = cameraId;
                return;
            }
        } catch (CameraAccessException e) {
            e.printStackTrace();
        } catch (NullPointerException e) {
            Log.e("###", "npe thrown");
        }
    }

    private void configureTransform(int viewWidth, int viewHeight) {
        Activity activity = this;
        if (null == mPreviewSize || null == activity) {
            return;
        }
        int rotation = activity.getWindowManager().getDefaultDisplay().getRotation();
        Matrix matrix = new Matrix();
        RectF viewRect = new RectF(0, 0, viewWidth, viewHeight);
        RectF bufferRect = new RectF(0, 0, mPreviewSize.getHeight(), mPreviewSize.getWidth());
        float centerX = viewRect.centerX();
        float centerY = viewRect.centerY();
        if (Surface.ROTATION_90 == rotation || Surface.ROTATION_270 == rotation) {
            bufferRect.offset(centerX - bufferRect.centerX(), centerY - bufferRect.centerY());
            matrix.setRectToRect(viewRect, bufferRect, Matrix.ScaleToFit.FILL);
            float scale = Math.max(
                    (float) viewHeight / mPreviewSize.getHeight(),
                    (float) viewWidth / mPreviewSize.getWidth());
            matrix.postScale(scale, scale, centerX, centerY);
            matrix.postRotate(90 * (rotation - 2), centerX, centerY);
        } else if (Surface.ROTATION_180 == rotation) {
            matrix.postRotate(180, centerX, centerY);
        }
        //TODO well....
        //mTextureView.setTransform(matrix);
    }

    private static Size chooseOptimalSize(Size[] choices, int textureViewWidth,
                                          int textureViewHeight, int maxWidth, int maxHeight, Size aspectRatio) {

        // Collect the supported resolutions that are at least as big as the preview Surface
        List<Size> bigEnough = new ArrayList<>();
        // Collect the supported resolutions that are smaller than the preview Surface
        List<Size> notBigEnough = new ArrayList<>();
        int w = aspectRatio.getWidth();
        int h = aspectRatio.getHeight();
        for (Size option : choices) {
            if (option.getWidth() <= maxWidth && option.getHeight() <= maxHeight &&
                    option.getHeight() == option.getWidth() * h / w) {
                if (option.getWidth() >= textureViewWidth &&
                        option.getHeight() >= textureViewHeight) {
                    bigEnough.add(option);
                } else {
                    notBigEnough.add(option);
                }
            }
        }

        // Pick the smallest of those big enough. If there is no one big enough, pick the
        // largest of those not big enough.
        if (bigEnough.size() > 0) {
            return Collections.min(bigEnough, new CompareSizesByArea());
        } else if (notBigEnough.size() > 0) {
            return Collections.max(notBigEnough, new CompareSizesByArea());
        } else {
            Log.e("XXX", "Couldn't find any suitable preview size");
            return choices[0];
        }
    }

    static class CompareSizesByArea implements Comparator<Size> {

        @Override
        public int compare(Size lhs, Size rhs) {
            // We cast here to ensure the multiplications won't overflow
            return Long.signum((long) lhs.getWidth() * lhs.getHeight() -
                    (long) rhs.getWidth() * rhs.getHeight());
        }

    }

/*
END Camera
*/



/*
START Simulation Settings
 */

    public static native void postConfig(String string);

    @Override
    public void updateSettings(String simulationSettings) {
        if (active) {
            String simSettings = EDSettingsController.getSimulationSettings();
            simSettings = new StringBuilder(simSettings).insert(simSettings.length() - 1, String.format(",\"xres\":%d,\"yres\":%d,\"rotation\":%d",width, height,mSensorOrientation)).toString();
            postConfig(simSettings);        }
    }

/*
END Simulation Settings
 */
}
