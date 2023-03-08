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
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;
import androidx.slidingpanelayout.widget.SlidingPaneLayout;

import com.google.android.material.floatingactionbutton.FloatingActionButton;
import com.vss.personas.Persona;
import com.vss.personas.PersonasAdapter;
import com.vss.simulator.SimulatorSurfaceView;

/**
 * Main activity.
 */
public class MainActivity extends AppCompatActivity implements ActivityCompat.OnRequestPermissionsResultCallback {

    private static final String LOG_TAG = "MainActivity";
    private static final int CAMERA_REQUEST_CODE = 100;

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
            StrictMode.setVmPolicy(new StrictMode.VmPolicy.Builder()
                    .detectAll().penaltyLog()
                    .build());
            StrictMode.setThreadPolicy(new StrictMode.ThreadPolicy.Builder()
                    .detectAll().permitDiskReads()
                    .penaltyFlashScreen().penaltyLog().build());
        }

        // Setup UI elements.
        this.personaSimulationPane = findViewById(R.id.persona_simulator_pane);
        this.startButton = findViewById(R.id.start_button);
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

    private void checkCameraPermission() {
        if (ActivityCompat.checkSelfPermission(this,
                Manifest.permission.CAMERA) == PackageManager.PERMISSION_DENIED) {
            ActivityCompat.requestPermissions(this,
                    new String[]{Manifest.permission.CAMERA}, CAMERA_REQUEST_CODE);
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

        Toast.makeText(this, R.string.start_simulator, Toast.LENGTH_LONG).show();

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
                Log.i("Camera", "Frame available");
                simulatorView.postFrame(width, height, y, u, v);
            }
        });

        // Prevent screen from turning off.
        getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);

        // Switch to simulator (low-resolution only)
        this.personaSimulationPane.open();
        //this.simulatorView.postSettings(jsonString);
        //this.simulatorView.start()

        this.startButton.hide();
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

    /*
      // Copy planes to buffers.
                image.getPlanes()[0].getBuffer().get(y);
                image.getPlanes()[1].getBuffer().get(u);
                image.getPlanes()[2].getBuffer().get(v);

                //TODO: simulatorView.postFrame(width, height, y, u, v);
     */

    //endregion
}
