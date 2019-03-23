package de.uni_stuttgart.vss.activities;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.support.v4.app.ActivityCompat;
import android.util.Log;
import android.widget.TextView;

import de.uni_stuttgart.vss.R;

/**
 * load the core library and hands off to the native activity
 */
public class CoreLoaderActivity extends Activity implements ActivityCompat.OnRequestPermissionsResultCallback {

    /**
     * stores if simulation core successfully loaded
     */
    private static boolean simCoreLoaded = false;

    //try to load core library
    static {
        Log.d("CoreLoaderActivity", "Loading Simulation-Core library ...");
        try {

            //load core library
            System.loadLibrary("core");
            simCoreLoaded = true;

            Log.d("CoreLoaderActivity", "Loading Simulation-Core library successful!");
        } catch (Exception e) {
            Log.d("CoreLoaderActivity", "Loading Simulation-Core library failed!", e);
        }
    }

    /**
     * create native activity
     *
     * @param savedInstanceState unused
     */
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        //set layout
        setContentView(R.layout.activity_core_loader);
        TextView text = findViewById(R.id.core_loader_text);

        //Simulation-Core loaded
        if (simCoreLoaded) {

            Log.d("CoreLoaderActivity", "Changing into native activity ...");

            //set text that "simulation core is loading"
            text.setText(R.string.core_loader_text_load_successful);

            try {
                //changing activity
                startActivity(new Intent(getBaseContext(), CoreActivity.class));

                Log.d("CoreLoaderActivity", "Changing into native activity successful!");

            } catch (Exception e) {

                Log.d("CoreLoaderActivity", "Changing into native activity failed!", e);

                //set text that "loading simulation core failed"
                text.setText(R.string.core_loader_text_load_failed);
            }
        }

        //Simulation-Core not loaded
        else {

            //set text that "loading simulation core failed"
            text.setText(R.string.core_loader_text_load_failed);
        }
    }
}
