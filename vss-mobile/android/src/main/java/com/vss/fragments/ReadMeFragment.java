package com.vss.fragments;

import android.annotation.SuppressLint;
import android.os.Bundle;
import androidx.annotation.NonNull;
import androidx.fragment.app.Fragment;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.webkit.WebView;

import com.vss.R;

/**
 * fragment containing the ReadMe-web-view
 */
public class ReadMeFragment extends Fragment {

    /**
     * initializes read-me-web-view
     */
    @SuppressLint("ValidFragment")
    public ReadMeFragment() {

        Log.d("ReadMeWebFragment", "START initializing ReadMeWebFragment");

        Log.d("ReadMeWebFragment", "END initializing ReadMeWebFragment");
    }

    /**
     * create the web-view
     *
     * @param inflater           to load ReadMe-view-layout
     * @param container          to load ReadMe-view-layout
     * @param savedInstanceState unused
     * @return created web-view
     */
    @Override
    public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {

        Log.d("ReadMeWebFragment", "START creating ReadMeWebView");

        //create web-view
        View view = inflater.inflate(R.layout.fragment_edcontroll, container, false);
        WebView webView = view.findViewById(R.id.wv_edcontroll);

        //manipulate view - load html-file, enable java-script and load java-script-interface
        webView.loadUrl("file:///android_asset/knowledgebase/README_GER.html");

        Log.d("ReadMeWebFragment", "END creating ReadMeWebView");

        //return created view
        return view;
    }
}