package com.vss.fragments;

import android.os.Bundle;
import androidx.annotation.NonNull;
import androidx.fragment.app.Fragment;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;

import com.vss.R;

/**
 * Fragment encapsulating simulation-display-related code.
 */
public class SimulatorFragment extends Fragment {

    @Override
    public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {

        //create view
        View view = inflater.inflate(R.layout.fragment_welcome, container, false);


        //return created view
        return view;
    }
}
