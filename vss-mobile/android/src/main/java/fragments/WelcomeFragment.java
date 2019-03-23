package de.uni_stuttgart.vss.fragments;

import android.os.Bundle;
import android.support.annotation.NonNull;
import android.support.v4.app.Fragment;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;

import de.uni_stuttgart.vss.R;

/**
 * fragment load at the app-start
 */
public class WelcomeFragment extends Fragment {

    /**
     * create the welcome-view
     *
     * @param inflater           to load welcome-view-layout
     * @param container          to load welcome-view-layout
     * @param savedInstanceState unused
     * @return created view
     */
    @Override
    public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {
        
        Log.d("WelcomeFragment", "START creating WelcomeView");

        //create view
        View view = inflater.inflate(R.layout.fragment_welcome, container, false);

        Log.d("WelcomeFragment", "END creating WelcomeView");

        //return created view
        return view;
    }
}
