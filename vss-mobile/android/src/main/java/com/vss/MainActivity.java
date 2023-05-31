package com.vss;

import android.Manifest;
import android.app.AlertDialog;
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

import java.util.concurrent.Callable;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.FutureTask;

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
            inspectorView.loadUrl("file:///android_asset/index.html");
            activityState = ActivityState.Welcome;
        } else {
            super.onBackPressed();
        }
    }

    //endregion

    //region Android permissions

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

    private void checkCameraPermission() {
        if (ActivityCompat.checkSelfPermission(this, Manifest.permission.CAMERA) == PackageManager.PERMISSION_DENIED) {
            ActivityCompat.requestPermissions(this, new String[]{Manifest.permission.CAMERA}, CAMERA_REQUEST_CODE);
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
                if (request.getUrl().getPath().endsWith("index.html")) {
                    activityState = ActivityState.Welcome;
                } else {
                    activityState = ActivityState.Inspecting;
                }
                // Allow local files and suppress other URLs.
                Log.i(LOG_TAG, "Loading URL: " + url);
                return url.getScheme() != "file";
            }
        });

        // Add JavaScript callback.
        inspectorView.getSettings().setJavaScriptEnabled(true);
        inspectorView.addJavascriptInterface(this, "Activity");

        // Load welcome page.
        inspectorView.loadUrl("file:///android_asset/index.html");
        activityState = ActivityState.Welcome;
    }

    @JavascriptInterface
    public String querySettings() throws ExecutionException, InterruptedException {
        FutureTask<String> futureResult = new FutureTask<String>(new Callable<String>() {
            @Override
            public String call() throws Exception {
                return simulatorView.querySettings();
            }
        });

        runOnUiThread(futureResult);
        return futureResult.get();
    }

    @JavascriptInterface
    public void postSettings(String jsonString) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                simulatorView.postSettings(jsonString);
            }
        });
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
        WindowInsetsControllerCompat windowInsetsController = WindowCompat.getInsetsController(getWindow(), getWindow().getDecorView());
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
        WindowInsetsControllerCompat windowInsetsController = WindowCompat.getInsetsController(getWindow(), getWindow().getDecorView());
        windowInsetsController.show(WindowInsetsCompat.Type.systemBars());
        // Allow turning off the screen.
        getWindow().clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        // Switch to inspection state.
        this.inspectorSimulatorPane.close();
        this.startButton.show();
        activityState = ActivityState.Inspecting;
    }

    //endregion

    private enum ActivityState {
        Welcome, Simulating, Inspecting,
    }
}
