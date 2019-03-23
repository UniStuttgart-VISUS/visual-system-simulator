package de.uni_stuttgart.vss.fragments;

import android.annotation.SuppressLint;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.support.annotation.NonNull;
import android.support.v4.app.Fragment;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.webkit.JavascriptInterface;
import android.webkit.WebView;

import de.uni_stuttgart.vss.R;
import de.uni_stuttgart.vss.edsettings.EDSettingsController;
import de.uni_stuttgart.vss.edsettings.EDSettingsListener;

/**
 * fragment containing the simulation-settings-web-view and
 * the interface between java-script in the sim-set-web-view and java
 */
@SuppressLint("ValidFragment")
public class EDControlFragment extends Fragment implements EDSettingsListener {

    /**
     * interface name for java script in web-view
     */
    private static final String INTERFACE_NAME = "Android";

    /**
     * reference web-view
     */
    private WebView webView;

    /**
     * reference to simulation settings controller
     */
    private EDSettingsController edSettingsController;

    /**
     * initializes simulation settings fragment
     * add fragment to simulation settings listeners
     *
     * @param edSettingsController simulation settings controller connected to the fragment
     */
    @SuppressLint("ValidFragment")
    public EDControlFragment(EDSettingsController edSettingsController) {

        Log.d("EDControlFragment", "START initializing EDControlFragment");

        //connectToServerFromRes to simulation settings controller
        this.edSettingsController = edSettingsController;
        this.edSettingsController.addListener(this);

        Log.d("EDControlFragment", "END initializing EDControlFragment");
    }

    /**
     * create the web-view
     *
     * @param inflater           to load view-layout
     * @param container          to load view-layout
     * @param savedInstanceState unused
     * @return created web-view
     */
    @SuppressLint("SetJavaScriptEnabled")
    @Override
    public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {

        Log.d("EDControlFragment", "START creating EDControlWebView");

        //create view + web-view
        View view = inflater.inflate(R.layout.fragment_edcontroll, container, false);
        webView = view.findViewById(R.id.wv_edcontroll);

        //manipulate web-view - load html-file and enable java-script
        webView.getSettings().setJavaScriptEnabled(true);
        webView.loadUrl("file:///android_asset/ui/ed_control_androidapp.html");

        //update the simulation settings
        updateSettings(null);

        Log.d("EDControlFragment", "START creating EDControlWebView");

        //return created view
        return view;
    }

    /**
     * updates the simulation settings and reloads the view
     */
    @Override
    public void updateSettings(String simulationSettings) {

        new Handler(Looper.getMainLooper()).post(() -> {

            //only if view is already created
            if (webView != null) {

                Log.d("EDControlFragment", "START update settings");

                //load html-file ed_control
                webView.reload();

                //reconnect javascript-interface
                webView.removeJavascriptInterface(INTERFACE_NAME);
                webView.addJavascriptInterface(this, INTERFACE_NAME);

                Log.d("EDControlFragment", "END update settings");
            }
        });
    }

    /**
     * called from web-view when simulation setting has changed
     *
     * @param id    settings-id whose value has changed
     * @param value new value
     */
    @JavascriptInterface
    public void changeValues(String id, String value) {
        Log.d("EDControlFragment", "START change value of " + id + " to " + value);
        edSettingsController.updateSettings(this, id, value);
        Log.d("EDControlFragment", "END change value");
    }

    /**
     * called from web-view when eye disease turned on
     *
     * @param id settings-id witch turned on
     */
    @JavascriptInterface
    public void activateED(String id) {
        Log.d("EDControlFragment", "START activating eye disease " + id);
        edSettingsController.activateED(this, id);
        Log.d("EDControlFragment", "END activating eye disease " + id);
    }

    /**
     * called from web-view when eye disease turned off
     *
     * @param id settings-id witch turned off
     */
    @JavascriptInterface
    public void deactivateED(String id) {
        Log.d("EDControlFragment", "START deactivating eye disease " + id);
        edSettingsController.deactivateED(this, id);
        Log.d("EDControlFragment", "END deactivating eye disease " + id);
    }

    /**
     * called from web-view to load boolean-value
     *
     * @param id settings-id of value to load
     * @return value
     */
    @JavascriptInterface
    public boolean getBooleanValue(String id) {
        return this.edSettingsController.getBooleanValue(id);
    }

    /**
     * called from web-view to load long-value
     *
     * @param id settings-id of value to load
     * @return value
     */
    @JavascriptInterface
    public long getLongValue(String id) {
        return this.edSettingsController.getLongValue(id);
    }
}
