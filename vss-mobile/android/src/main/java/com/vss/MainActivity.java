package com.vss;

import android.Manifest;
import android.app.AlertDialog;
import android.content.pm.ActivityInfo;
import android.content.pm.PackageManager;
import android.hardware.camera2.CameraDevice;
import android.net.Uri;
import android.os.Bundle;
import android.os.StrictMode;
import android.util.Log;
import android.view.View;
import android.view.WindowManager;
import android.webkit.JavascriptInterface;
import android.webkit.WebResourceRequest;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.EditText;
import android.widget.Toast;

import androidx.annotation.NonNull;
import androidx.appcompat.app.AppCompatActivity;
import androidx.core.app.ActivityCompat;
import androidx.core.view.WindowCompat;
import androidx.core.view.WindowInsetsCompat;
import androidx.core.view.WindowInsetsControllerCompat;
import androidx.slidingpanelayout.widget.SlidingPaneLayout;

import com.google.android.material.floatingactionbutton.FloatingActionButton;
import com.vss.simulator.SimulatorSurfaceView;

import java.io.File;
import java.net.URI;
import java.util.Locale;

/**
 * Main activity.
 */
public class MainActivity extends AppCompatActivity implements ActivityCompat.OnRequestPermissionsResultCallback {
    private static final String LOG_TAG = "MainActivity";
    private static final int CAMERA_REQUEST_CODE = 100;

    private ActivityState activityState = ActivityState.Welcome;
    private SlidingPaneLayout inspectorSimulatorPane;
    private FloatingActionButton startButton;

    private WebView inspectorView;

    private SimulatorSurfaceView simulatorView;
    private CameraAccess cameraAccess;

    //region Android activity lifecycle

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);

        // Test for debug mode.
        if (BuildConfig.DEBUG) {
            Log.w(LOG_TAG, "======= APPLICATION IN STRICT MODE - DEBUGGING =======");
            StrictMode.setVmPolicy(new StrictMode.VmPolicy.Builder().detectAll().penaltyLog().build());
            StrictMode.setThreadPolicy(new StrictMode.ThreadPolicy.Builder().detectAll().permitDiskReads().penaltyFlashScreen().penaltyLog().build());
        }

        // Setup UI elements.
        setupInspectorSimulatorChanger();
        setupInspectorView();
        this.simulatorView = findViewById(R.id.simulator_view);
    }

    @Override
    public void onBackPressed() {
        if (activityState == ActivityState.Simulating) {
            stopSimulator();
        } else if (activityState == ActivityState.Inspecting) {
            loadPage("index.html");
        } else {
            super.onBackPressed();
        }
    }

    //endregion

    //region Android permissions

    @Override
    public void onRequestPermissionsResult(int requestCode, @NonNull String[] permissions,
                                           @NonNull int[] grantResults) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        if (requestCode == CAMERA_REQUEST_CODE) {
            if (grantResults[0] == PackageManager.PERMISSION_GRANTED) {
                Log.w("Permission", "Camera: GRANTED");
            } else {
                Log.i("Permission", "Camera: DENIED");
            }
        }
    }

    private void checkCameraPermission() {
        if (ActivityCompat.checkSelfPermission(this, Manifest.permission.CAMERA) == PackageManager.PERMISSION_DENIED) {
            ActivityCompat.requestPermissions(this, new String[]{Manifest.permission.CAMERA},
                    CAMERA_REQUEST_CODE);
        }
    }

    //endregion

    //region Inspector-Simulation Changer

    private void setupInspectorSimulatorChanger() {
        this.inspectorSimulatorPane = findViewById(R.id.inspector_simulator_pane);

        // Suppress user swipes (we use the start button instead).
        this.inspectorSimulatorPane.setLockMode(SlidingPaneLayout.LOCK_MODE_LOCKED);

        this.startButton = findViewById(R.id.start_button);
    }

    public void startStopClicked(View view) {
        this.startSimulator();
    }

    //endregion

    //region Inspector

    private void setupInspectorView() {
        inspectorView = findViewById(R.id.inspector_view);

        // Configure WebView.
        inspectorView.setHorizontalScrollBarEnabled(false);

        // Intercept URL loading.
        inspectorView.setWebViewClient(new WebViewClient() {
            @Override
            public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest request) {
                final Uri url = request.getUrl();
                // Allow local files and suppress other URLs.
                Log.i(LOG_TAG, "Loading URL: " + url);
                return url.getScheme() != "file";
            }
        });

        // Add JavaScript callback.
        inspectorView.getSettings().setJavaScriptEnabled(true);
        inspectorView.addJavascriptInterface(this, "Activity");

        // Load welcome page.
        loadPage("index.html");
    }

    @JavascriptInterface
    public void loadPage(String pageName) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                if (pageName == "index.html") {
                    activityState = ActivityState.Welcome;
                } else {
                    activityState = ActivityState.Inspecting;
                }
                Locale currentLocale = getResources().getConfiguration().getLocales().get(0);
                String isoName = currentLocale.getISO3Country();
                String url = "file:///android_asset/" + isoName + "/" + pageName;
                File file = new File(URI.create(isoName).getPath());
                if (file.exists()) {
                    Log.i(LOG_TAG, "Loading " + url);
                    inspectorView.loadUrl(url);
                } else {
                    String urlFallback = "file:///android_asset/en/" + pageName;
                    Log.w(LOG_TAG, "Loading fallback " + urlFallback);
                    inspectorView.loadUrl(urlFallback);
                }
            }
        });
    }

    @JavascriptInterface
    public void setJSONSettings(String jsonString) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                simulatorView.postSettings(jsonString);
            }
        });
    }

    @JavascriptInterface
    public String getJSONSettings(String jsonString) {
        return simulatorView.querySettings();
    }

    //endregion

    //region Simulator

    /**
     * Starts the simulator.
     */
    public void startSimulator() {
        Log.d(LOG_TAG, "Starting simulator");

        // Open camera.
        this.cameraAccess = new CameraAccess(this, new CameraAccess.CameraDelegate() {
            @Override
            public void onCameraOpen(CameraDevice cameraDevice) {
            }

            @Override
            public void onCameraDisconnected(CameraDevice cameraDevice) {
            }

            @Override
            public void onCameraError(CameraDevice cameraDevice, int error) {
            }

            @Override
            public void onCameraPermissionDenied() {
                checkCameraPermission();
            }

            @Override
            public void onFrameAvailable(int width, int height, byte[] y, byte[] u, byte[] v) {
                simulatorView.postFrame(width, height, y, u, v);
            }
        });

        // Enter immersive mode.
        WindowInsetsControllerCompat windowInsetsController =
                WindowCompat.getInsetsController(getWindow(), getWindow().getDecorView());
        windowInsetsController.setSystemBarsBehavior(WindowInsetsControllerCompat.BEHAVIOR_SHOW_BARS_BY_TOUCH);
        windowInsetsController.hide(WindowInsetsCompat.Type.systemBars());
        // Prevent screen from turning off.
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        // Switch to simulation state.
        this.inspectorSimulatorPane.open();
        this.startButton.hide();
        activityState = ActivityState.Simulating;
    }

    void stopSimulator() {
        // Close camera.
        this.cameraAccess.close();
        this.cameraAccess = null;

        // Leave immersive mode.
        WindowInsetsControllerCompat windowInsetsController =
                WindowCompat.getInsetsController(getWindow(), getWindow().getDecorView());
        windowInsetsController.show(WindowInsetsCompat.Type.systemBars());
        // Allow turning off the screen.
        getWindow().clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        // Switch to inspection state.
        this.inspectorSimulatorPane.close();
        this.startButton.show();
        activityState = ActivityState.Inspecting;
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
        new AlertDialog.Builder(this).setTitle(R.string.settings_save).setMessage(R.string.save_as).setView(input).setPositiveButton(R.string.store, (dialogInterface, i) -> {
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
        new AlertDialog.Builder(this).setTitle(R.string.settings_reset).setMessage(R.string.eyediseases_settings_reset_confirmation).setPositiveButton(R.string.settings_reset, (dialogInterface, i) -> {
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
        Log.d(LOG_TAG, "Loading simulator settings...");

        //TODO: load simulator settings from id

        //print message
        Toast.makeText(this, R.string.eyediseases_settings_load_successful, Toast.LENGTH_SHORT).show();

        Log.d(LOG_TAG, "Loading simulator settings successful!");
    }

    /**
     * Save simulator settings.
     *
     * @param name name of simulator settings
     */
    private void saveSimulatorSettings(String name) {
        Log.d(LOG_TAG, "Saving simulator settings ...");

        //TODO: store simulator settings with name

        Toast.makeText(this, R.string.eyediseases_settings_store_successful, Toast.LENGTH_SHORT).show();

        Log.d(LOG_TAG, "Saving simulator settings successful!");
    }

    /**
     * Resets the simulator settings
     */
    private void resetSimulatorSettings() {
        Log.d(LOG_TAG, "Resetting simulator settings...");

        //TODO: load default settings

        Toast.makeText(this, R.string.eyediseases_settings_reset_successful, Toast.LENGTH_SHORT).show();

        Log.d(LOG_TAG, "Resetting simulator settings successful!");
    }

    //endregion

    private enum ActivityState {
        Welcome, Simulating, Inspecting,
    }
}
