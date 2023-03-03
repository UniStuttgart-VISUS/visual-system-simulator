package com.vss;

import static android.hardware.camera2.CameraDevice.TEMPLATE_PREVIEW;

import android.Manifest;
import android.app.AlertDialog;
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
import android.net.Uri;
import android.os.Bundle;
import android.os.StrictMode;
import android.util.Log;
import android.util.Size;
import android.view.View;
import android.view.WindowManager;
import android.webkit.WebResourceRequest;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.EditText;
import android.widget.Toast;

import androidx.annotation.NonNull;
import androidx.appcompat.app.AppCompatActivity;
import androidx.core.app.ActivityCompat;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;
import androidx.slidingpanelayout.widget.SlidingPaneLayout;

import com.vss.personas.Persona;
import com.vss.personas.PersonasAdapter;
import com.vss.simulator.SimulatorSurfaceView;

import java.util.Arrays;
import java.util.Collections;
import java.util.Comparator;
import java.util.List;

/**
 * Main activity.
 */
public class MainActivity extends AppCompatActivity implements ActivityCompat.OnRequestPermissionsResultCallback {

    private static final String LOG_TAG = "MainActivity";
    private static final int CAMERA_REQUEST_CODE = 100;

    private SlidingPaneLayout personaSimulationPane;

    private PersonasAdapter personasAdapter;
    private RecyclerView personasView;

    private WebView inspectorView;

    private SimulatorSurfaceView simulatorView;
    private ImageReader imageReader;

    //region Android activity lifecycle

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);

        // Test for debug mode.
        if (BuildConfig.DEBUG) {
            Log.w(LOG_TAG, "======= APPLICATION IN STRICT MODE - DEBUGGING =======");
            StrictMode.setVmPolicy(new StrictMode.VmPolicy.Builder()
                    .detectAll().penaltyLog()
                    .build());
            StrictMode.setThreadPolicy(new StrictMode.ThreadPolicy.Builder()
                    .detectAll().permitDiskReads()
                    .penaltyFlashScreen().penaltyLog().build());
        }

        // Setup UI elements.
        this.personaSimulationPane = findViewById(R.id.persona_simulator_pane);
        setupPersonasView();
        setupInspectorView();
        this.simulatorView = findViewById(R.id.simulator_view);
    }

    @Override
    public void onBackPressed() {
       /* if (((DrawerLayout) findViewById(R.id.drawer_layout)).isDrawerOpen(GravityCompat.START)) {
            closeNavMenu();
        } else {
            switch (fragmentState) {
                case READ_ME:
                case WEBUI_SETTINGS:
                case KNOWLEDGEBASE_LIST:
                    openEDSettingsFragment();
                    break;
                case KNOWLEDGEBASE_CONTENT:
                    openKnowledgebaseListFragment();
                    break;
                case ED_SETTINGS:
                default:
                    super.onBackPressed();
                    break;
            }
        }
        */
    }

    //endregion

    //region Android permissions

    private boolean checkCameraPermission(boolean requestIfMissing) {
        if (ActivityCompat.checkSelfPermission(this,
                Manifest.permission.CAMERA) == PackageManager.PERMISSION_DENIED) {
            if (requestIfMissing) {
                ActivityCompat.requestPermissions(this,
                        new String[]{Manifest.permission.CAMERA}, CAMERA_REQUEST_CODE);
            }
            return false;
        } else {
            return true;
        }
    }

    @Override
    public void onRequestPermissionsResult(int requestCode, @NonNull String[] permissions, @NonNull int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        if (requestCode == CAMERA_REQUEST_CODE) {
            if (grantResults[0] == PackageManager.PERMISSION_GRANTED) {
                Log.w("Permission", "Camera: GRANTED");
            } else {
                Log.i("Permission", "Camera: DENIED");
            }
        }
    }

    //endregion

    //region Personas

    private void setupPersonasView() {
        Persona[] personas = {new Persona("yyy", "Custom", "<html>"), new Persona("xxx", "Achromataposie", "<html>"), new Persona("xxx", "Ametropie", "<html>"), new Persona("xxx", "Katarrakt", "<html>"), new Persona("xxx", "Dyschro...", "<html>"), new Persona("xxx", "Glaukom", "<html>"), new Persona("xxx", "X", "<html>"), new Persona("xxx", "Y", "<html>"), new Persona("xxx", "Z", "<html>")};
        personasAdapter = new PersonasAdapter(personas);

        personasView = findViewById(R.id.preset_view);
        personasView.setHasFixedSize(true);
        LinearLayoutManager layoutManager = new LinearLayoutManager(this);
        layoutManager.setOrientation(LinearLayoutManager.HORIZONTAL);
        personasView.setLayoutManager(layoutManager);
        personasView.setAdapter(personasAdapter);
    }

    //endregion

    //region Inspector

    private void setupInspectorView() {
        inspectorView = findViewById(R.id.inspector_view);

        // Intercept URL loading.
        inspectorView.setWebViewClient(new WebViewClient() {
            @Override
            public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
                final Uri url = request.getUrl();
                // Allow local files.
                // Suppress other links.
                return url.getScheme() != "file";
            }
        });

        // Add JavaScript callback.
        inspectorView.getSettings().setJavaScriptEnabled(true);
        inspectorView.addJavascriptInterface(this.simulatorView, "SimulatorView");

        //TODO: add event listener for "config changed/loaded" or something like that?

        // Load welcome page.
        inspectorView.loadUrl("file:///android_asset/welcome.html");
    }

    //endregion

    //region Simulator

    /**
     * Starts the simulator.
     */
    public void startSimulator(View view) {
        Log.d(LOG_TAG, "Starting simulator");

        if (checkCameraPermission(true)) {
            Toast.makeText(this, R.string.start_simulator, Toast.LENGTH_LONG).show();

            setupCamera();

            // Prevent screen from turning off.
            getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

            // Switch to simulator (low-resolution only)
            this.personaSimulationPane.open();
            //this.simulatorView.postSettings(jsonString);
            //this.simulatorView.start()
        } else {
            Toast.makeText(this, "Camera permission required", Toast.LENGTH_SHORT).show();
        }
    }

    /**
     * Dialog to save simulator settings.
     */
    private void showSaveSimulatorSettingsDialog() {
        Log.d(LOG_TAG, "OPEN save simulator settings dialog");

        // Create dialog contents.
        EditText input = new EditText(this);
        input.setSingleLine();

        // Show dialog.
        new AlertDialog.Builder(this).setTitle(R.string.eyediseases_settings_store).setMessage(R.string.save_as).setView(input).setPositiveButton(R.string.store, (dialogInterface, i) -> {
            saveSimulatorSettings(input.getText().toString());
        }).setNegativeButton(R.string.cancel, null).setOnDismissListener(dialogInterface -> {
            Log.d(LOG_TAG, "CLOSE save simulator settings dialog");
        }).show();

    }

    /**
     * Dialog to confirm resetting simulator settings.
     */
    private void showResetSimulatorSettingsDialog() {
        Log.d(LOG_TAG, "OPEN reset simulator settings dialog");

        // Show dialog.
        new AlertDialog.Builder(this).setTitle(R.string.eyediseases_settings_reset).setMessage(R.string.eyediseases_settings_reset_confirmation).setPositiveButton(R.string.eyediseases_settings_reset, (dialogInterface, i) -> {
            resetSimulatorSettings();
        }).setNegativeButton(R.string.cancel, null).setOnDismissListener(dialogInterface -> {
            Log.d(LOG_TAG, "CLOSE reset simulator settings dialog");
        }).show();
    }

    /**
     * Load simulator settings.
     *
     * @param id settings-id
     */
    private void loadSimulatorSettings(int id) {
        Log.d(LOG_TAG, "Load simulator settings ...");

        //try to load simulator settings
        //   try {

        //load simulator settings from id
        // this.edSettingsController.loadSimulatorSettings(id);

        //print message
        Toast.makeText(this, R.string.eyediseases_settings_load_successful, Toast.LENGTH_SHORT).show();

        Log.d(LOG_TAG, "Load simulator settings successful!");
        // }

        //loading failed
        // catch (IOException | IndexOutOfBoundsException e) {

        //print message
        //    Toast.makeText(this, R.string.eyediseases_settings_load_failed, Toast.LENGTH_SHORT).show();

        //   Log.d("MainMenu", "Load simulator settings failed!", e);
        //}
    }

    /**
     * Save simulator settings.
     *
     * @param name name of simulator settings
     */
    private void saveSimulatorSettings(String name) {
        Log.d(LOG_TAG, "Store simulator settings ...");

        //try to store simulator settings
        //  try {

        //store simulator settings with name
        //   edSettingsController.storeSimulatorSettings(name);

        //print message
        Toast.makeText(this, R.string.eyediseases_settings_store_successful, Toast.LENGTH_SHORT).show();

        Log.d(LOG_TAG, "Store simulator settings successful!");
        //  }

        //storing failed
        //  catch (IOException e) {

        //print message
        //     Toast.makeText(this, R.string.eyediseases_settings_store_failed, Toast.LENGTH_SHORT).show();

        //     Log.d("MainMenu", "Store simulator settings failed", e);
        //  }
    }

    /**
     * Resets the simulator settings
     */
    private void resetSimulatorSettings() {
        Log.d(LOG_TAG, "Reset simulator settings ...");

        //try to reset simulator settings
        //  try {

        //load default simulator settings
        //    edSettingsController.loadDefaultSimulatorSettings();

        //print message
        Toast.makeText(this, R.string.eyediseases_settings_reset_successful, Toast.LENGTH_SHORT).show();

        Log.d(LOG_TAG, "Reset simulator settings successful!");
        // }

        //reset failed
        //  catch (IOException e) {

        //print message
        //    Toast.makeText(this, R.string.eyediseases_settings_reset_failed, Toast.LENGTH_SHORT).show();

        //     Log.d("MainMenu", "Reset simulator settings failed!", e);
        //  }
    }

    //endregion


    //region Camera

    private void setupCamera() {
        if (!checkCameraPermission(false)) {
            return;
        }

        CameraManager manager = (CameraManager) this.getSystemService(Context.CAMERA_SERVICE);
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
                        setupCameraSession(cameraDevice, imageFormat, largestSize);

                    }

                    @Override
                    public void onDisconnected(CameraDevice cameraDevice) {
                        Log.i("Camera", "Device disconnected");
                    }

                    @Override
                    public void onError(CameraDevice cameraDevice, int error) {

                        Log.e("Camera", "Device error (" + error + ")");
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

                // Post frame to simulator.
                simulatorView.postFrame(width, height, y, u, v);

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

    //endregion
}
