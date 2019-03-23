package de.uni_stuttgart.vss.fragments;

import android.annotation.SuppressLint;
import android.os.Bundle;
import android.support.annotation.NonNull;
import android.support.v4.app.Fragment;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.webkit.JavascriptInterface;
import android.webkit.WebView;

import java.io.IOException;

import de.uni_stuttgart.vss.KnowledgebaseContentItem;
import de.uni_stuttgart.vss.R;
import de.uni_stuttgart.vss.edsettings.EDSettingsController;

/**
 * fragment containing the knowledgebase-web-view
 */
@SuppressLint("ValidFragment")
public class KnowledgebaseWebFragment extends Fragment {

    /**
     * displayed knowledgebase entry
     */
    private final KnowledgebaseContentItem contentItem;

    /**
     * reference to simulation settings controller
     */
    private EDSettingsController edSettingsController;

    /**
     * initializes knowledgebase-web-view
     *
     * @param edSettingsController simulation settings controller connected to the fragment
     * @param item                 knowledgebase entry to display
     */
    @SuppressLint("ValidFragment")
    public KnowledgebaseWebFragment(EDSettingsController edSettingsController, KnowledgebaseContentItem item) {

        Log.d("KnowledgebaseWebFragment", "START initializing KnowledgebaseWebFragment");

        this.edSettingsController = edSettingsController;
        this.contentItem = item;

        Log.d("KnowledgebaseWebFragment", "END initializing KnowledgebaseWebFragment");
    }

    /**
     * create the web-view
     *
     * @param inflater           to load knowledgebase-view-layout
     * @param container          to load knowledgebase-view-layout
     * @param savedInstanceState unused
     * @return created web-view
     */
    @SuppressLint("SetJavaScriptEnabled")
    @Override
    public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {

        Log.d("KnowledgebaseWebFragment", "START creating KnowledgebaseWebView");

        //create web-view
        View view = inflater.inflate(R.layout.fragment_edcontroll, container, false);
        WebView webView = view.findViewById(R.id.wv_edcontroll);

        //manipulate view - load html-file, enable java-script and load java-script-interface
        webView.getSettings().setJavaScriptEnabled(true);
        webView.loadUrl(contentItem.contentUrl);
        webView.addJavascriptInterface(this, "Android");

        Log.d("KnowledgebaseWebFragment", "END creating KnowledgebaseWebView");

        //return created view
        return view;
    }

    /**
     * called from web-view to load a simulation settings preset
     *
     * @param file file to load the preset from
     */
    @JavascriptInterface
    public void loadSimulationSettings(String file) {

        Log.d("KnowledgebaseWebFragment", "Load preset simulation-setting (" + file + ")...");

        //try to load prepared simulation-settings
        try {

            //load prepared simulation-settings
            edSettingsController.loadPreparedSimulationSettings(file);

            Log.d("KnowledgebaseWebFragment", "Load preset simulation-setting successful!");
        }

        //load settings failed
        catch (IOException e) {

            Log.d("KnowledgebaseWebFragment", "Load preset simulation-setting failed!", e);
        }
    }

}