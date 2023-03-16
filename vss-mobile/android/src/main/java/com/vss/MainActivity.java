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
import androidx.recyclerview.widget.RecyclerView;
import androidx.slidingpanelayout.widget.SlidingPaneLayout;

import com.google.android.material.floatingactionbutton.FloatingActionButton;
import com.vss.personas.Persona;
import com.vss.personas.PersonasAdapter;
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

    private ActivityState activityState = ActivityState.Inspecting;
    private SlidingPaneLayout personaSimulationPane;
    private FloatingActionButton startButton;
    private PersonasAdapter personasAdapter;
    private RecyclerView personasView;

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
        setupPersonasSimulationChanger();
        setupPersonasView();
        setupInspectorView();
        this.simulatorView = findViewById(R.id.simulator_view);
    }

    @Override
    public void onBackPressed() {
        if (activityState == ActivityState.Simulating) {
            stopSimulator();
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

    //region Personas-Simulation

    private void setupPersonasSimulationChanger() {
        this.personaSimulationPane = findViewById(R.id.persona_simulator_pane);

        // Suppress user swipes (we use the start button instead).
        this.personaSimulationPane.setLockMode(SlidingPaneLayout.LOCK_MODE_LOCKED);

        this.startButton = findViewById(R.id.start_button);
    }

    public void startStopClicked(View view) {
        this.startSimulator();
    }

    //endregion

    //region Personas

    private void setupPersonasView() {
        //@formatter:off
        Persona[] personas = {
            new Persona("custom.html", "custom", "Custom"),
            new Persona("achromatopsia.html", "custom", "Achromatopsia"),
            new Persona("ametropia.html", "custom", "Ametropia"),
            new Persona("cataract.html", "custom", "Cataract"),
            new Persona("color-blindness.html", "custom", "Color Blindness"),
            new Persona("glaucoma.html", "custom", "Glaucoma"),
            new Persona("macular-degeneration.html", "custom", "Macular Degeneration"),
            new Persona("night-blindness.html", "custom", "Night Blindness"),
            new Persona("presbyopia.html", "custom", "Presbyopia")
        };
        //@formatter:on
        personasAdapter = new PersonasAdapter(personas, new PersonasAdapter.PersonasDelegate() {
            @Override
            public void onSelected(Persona persona) {
                loadInspectorPage(persona.pageName);
            }
        });

        personasView = findViewById(R.id.preset_view);
        personasView.setAdapter(personasAdapter);
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
        inspectorView.addJavascriptInterface(this.simulatorView, "SimulatorView");

        //TODO: add event listener for "config changed/loaded" or something like that?

        // Load welcome page.
        loadInspectorPage("index.html");
    }

    private void loadInspectorPage(String pageName) {
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

        // Setup simulator.
        //this.simulatorView.postSettings(jsonString);
        //this.simulatorView.start()

        // Enter immersive mode and prevent screen from turning off.
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        WindowInsetsControllerCompat windowInsetsController =
                WindowCompat.getInsetsController(getWindow(), getWindow().getDecorView());
        windowInsetsController.setSystemBarsBehavior(WindowInsetsControllerCompat.BEHAVIOR_SHOW_BARS_BY_TOUCH);
        windowInsetsController.hide(WindowInsetsCompat.Type.systemBars());

        // Switch to simulation state.
        this.personaSimulationPane.open();
        this.startButton.hide();
        activityState = ActivityState.Simulating;
    }

    void stopSimulator() {
        // Close camera.
        this.cameraAccess.close();
        this.cameraAccess = null;

        // Leave immersive mode and allow turning off the screen.
        WindowInsetsControllerCompat windowInsetsController =
                WindowCompat.getInsetsController(getWindow(), getWindow().getDecorView());
        windowInsetsController.show(WindowInsetsCompat.Type.systemBars());
        getWindow().clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        // Switch to inspection state.
        this.personaSimulationPane.close();
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
        //    Toast.makeText(this, R.string.eyediseases_settings_load_failed, Toast.LENGTH_SHORT)
        //    .show();

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
        //     Toast.makeText(this, R.string.eyediseases_settings_store_failed, Toast
        //     .LENGTH_SHORT).show();

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
        //    Toast.makeText(this, R.string.eyediseases_settings_reset_failed, Toast
        //    .LENGTH_SHORT).show();

        //     Log.d("MainMenu", "Reset simulator settings failed!", e);
        //  }
    }

    private enum ActivityState {
        Simulating, Inspecting,
    }

    /*
      // Copy planes to buffers.
                image.getPlanes()[0].getBuffer().get(y);
                image.getPlanes()[1].getBuffer().get(u);
                image.getPlanes()[2].getBuffer().get(v);

                //TODO: simulatorView.postFrame(width, height, y, u, v);
     */

    //endregion
}
