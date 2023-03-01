package com.vss.activities;

import android.app.Activity;
import android.os.Bundle;
import android.widget.TextView;

import com.vss.LibBridge;
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

        LibBridge bridge = new LibBridge();
        bridge.draw();

        if (LibBridge.tryLoadLibrary()) {
            statusText.setText(R.string.main_loader_status_successful);



           // startActivity(new Intent(getBaseContext(), MainActivity.class));
        } else {
             statusText.setText(R.string.main_loader_status_failed);
        }
    }

}
