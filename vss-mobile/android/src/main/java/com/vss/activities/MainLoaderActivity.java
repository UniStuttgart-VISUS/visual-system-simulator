package com.vss.activities;

import android.app.Activity;
import android.content.Intent;
import android.os.Bundle;
import android.util.Log;
import android.widget.TextView;

import com.vss.R;

/**
 * Loads the library and then hands off to the native activity.
 */
public class MainLoaderActivity extends Activity {
    private TextView statusText;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        setContentView(R.layout.activity_main_loader);
        statusText = findViewById(R.id.status_text);

        if (tryLoadNativeLibrary()) {
            startActivity(new Intent(getBaseContext(), MainActivity.class));
        }
    }

    private boolean tryLoadNativeLibrary() {
        try {
            Log.d("MainLoaderActivity", "Loading native library...");
            System.loadLibrary("vss");
            statusText.setText(R.string.main_loader_status_successful);
            Log.d("MainLoaderActivity", "Loading native library: successful");
            return true;
        } catch (java.lang.UnsatisfiedLinkError e) {
            statusText.setText(R.string.main_loader_status_failed);
            Log.e("MainLoaderActivity", "Loading native library: failed", e);
            return false;
        }
    }
}
