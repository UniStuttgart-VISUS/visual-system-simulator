package com.vss;

import android.Manifest;
import android.app.AlertDialog;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.hardware.HardwareBuffer;
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
    private static final int MEDIA_REQUEST_CODE = 101;

    private ActivityState activityState = ActivityState.Welcome;
    private SlidingPaneLayout inspectorSimulatorPane;
    private FloatingActionButton startButton;
    private FloatingActionButton galleryButton;

    private WebView inspectorView;

    private SimulatorSurfaceView simulatorView;
    private CameraFrameSource cameraFrameSource;
    private MediaFrameSource mediaFrameSource;

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
        handleSendMediaIntent(getIntent());
    }

    @Override
    protected void onNewIntent(Intent intent) {
        super.onNewIntent(intent);
        setIntent(intent);
        handleSendMediaIntent(intent);
    }

    @Override
    protected void onDestroy() {
        stopCamera();
        stopMedia();
        super.onDestroy();
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
        this.galleryButton = findViewById(R.id.gallery_button);
    }

    public void startStopClicked(View view) {
        this.startSimulator();
    }

    public void openMediaClicked(View view) {
        Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
        intent.addCategory(Intent.CATEGORY_OPENABLE);
        intent.setType("*/*");
        intent.putExtra(Intent.EXTRA_MIME_TYPES, new String[]{"image/*", "video/*"});
        intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
        startActivityForResult(intent, MEDIA_REQUEST_CODE);
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

        // Disable autofill.
        inspectorView.getSettings().setSaveFormData(false);

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

        stopMedia();
        startCamera();
        enterSimulationState();
    }

    private void startMedia(Uri uri, String mimeType) {
        Log.d(LOG_TAG, "Starting simulator from media: " + uri);

        stopCamera();
        enterSimulationState();
        if (this.mediaFrameSource == null) {
            this.mediaFrameSource = new MediaFrameSource(this, simulatorView);
        }
        this.mediaFrameSource.start(uri, mimeType);
    }

    private void enterSimulationState() {

        // Enter immersive mode.
        WindowInsetsControllerCompat windowInsetsController = WindowCompat.getInsetsController(getWindow(), getWindow().getDecorView());
        windowInsetsController.setSystemBarsBehavior(WindowInsetsControllerCompat.BEHAVIOR_SHOW_BARS_BY_TOUCH);
        windowInsetsController.hide(WindowInsetsCompat.Type.systemBars());
        // Prevent screen from turning off.
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        // Switch to simulation state.
        this.inspectorSimulatorPane.open();
        this.startButton.hide();
        this.galleryButton.hide();
        activityState = ActivityState.Simulating;
    }

    private void startCamera() {
        this.cameraFrameSource = new CameraFrameSource(this, new CameraFrameSource.CameraDelegate() {
            @Override
            public void onCameraPermissionDenied() {
                checkCameraPermission();
            }

            @Override
            public void onFrameAvailable(
                    int width,
                    int height,
                    int dataSpace,
                    int rotationDegrees,
                    HardwareBuffer hardwareBuffer
            ) {
                simulatorView.postHardwareBuffer(width, height, dataSpace, rotationDegrees, hardwareBuffer);
            }
        });
    }

    void stopSimulator() {
        stopCamera();
        stopMedia();

        // Leave immersive mode.
        WindowInsetsControllerCompat windowInsetsController = WindowCompat.getInsetsController(getWindow(), getWindow().getDecorView());
        windowInsetsController.show(WindowInsetsCompat.Type.systemBars());
        // Allow turning off the screen.
        getWindow().clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        // Switch to inspection state.
        this.inspectorSimulatorPane.close();
        this.startButton.show();
        this.galleryButton.show();
        activityState = ActivityState.Inspecting;
    }

    //endregion

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (requestCode != MEDIA_REQUEST_CODE || resultCode != RESULT_OK || data == null) {
            return;
        }

        Uri uri = data.getData();
        if (uri == null) {
            Log.w(LOG_TAG, "Media picker did not return a URI");
            return;
        }

        takeReadPermission(uri, data.getFlags());
        startMedia(uri, data.getType());
    }

    private void stopCamera() {
        if (this.cameraFrameSource != null) {
            this.cameraFrameSource.close();
            this.cameraFrameSource = null;
        }
    }

    private void stopMedia() {
        if (this.mediaFrameSource != null) {
            this.mediaFrameSource.close();
            this.mediaFrameSource = null;
        }
    }

    private void handleSendMediaIntent(Intent intent) {
        if (intent == null || !Intent.ACTION_SEND.equals(intent.getAction())) {
            return;
        }

        Uri uri = intent.getParcelableExtra(Intent.EXTRA_STREAM);
        if (uri == null) {
            Log.w(LOG_TAG, "Shared media intent did not contain a stream");
            return;
        }

        takeReadPermission(uri, intent.getFlags());
        startMedia(uri, intent.getType());
    }

    private void takeReadPermission(Uri uri, int flags) {
        try {
            getContentResolver().takePersistableUriPermission(
                    uri,
                    flags & Intent.FLAG_GRANT_READ_URI_PERMISSION
            );
        } catch (SecurityException err) {
            Log.d(LOG_TAG, "Media URI is not persistable; using transient grant");
        }
    }

    private enum ActivityState {
        Welcome, Simulating, Inspecting,
    }
}
