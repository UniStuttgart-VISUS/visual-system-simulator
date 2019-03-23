package de.uni_stuttgart.vss.fragments;

import android.annotation.SuppressLint;
import android.content.Context;
import android.os.Bundle;
import android.support.annotation.NonNull;
import android.support.v4.app.Fragment;
import android.support.v7.widget.LinearLayoutManager;
import android.support.v7.widget.RecyclerView;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;

import java.io.IOException;

import de.uni_stuttgart.vss.KnowledgebaseContentItem;
import de.uni_stuttgart.vss.KnowledgebaseListContent;
import de.uni_stuttgart.vss.R;

/**
 * fragment containing list of knowledgebase-items
 */
@SuppressLint("ValidFragment")
public class KnowledgebaseListFragment extends Fragment {

    /**
     * knowledgebase-content
     */
    private KnowledgebaseListContent knowledgebaseListContent;

    /**
     * default constructor
     */
    @SuppressLint("ValidFragment")
    public KnowledgebaseListFragment(Context context) throws IOException {

        Log.d("KnowledgebaseListFragment", "START initializing KnowledgebaseListFragment");

        //context is listener for knowledgebase-list
        if (context instanceof OnListFragmentInteractionListener) {

            //load listener
            OnListFragmentInteractionListener mListener = (OnListFragmentInteractionListener) context;

            //load content
            this.knowledgebaseListContent = new KnowledgebaseListContent(mListener, context);
        }

        //context is no listener for knowledgebase-list
        else {
            throw new RuntimeException(context.toString() + " must implement OnListFragmentInteractionListener");
        }

        Log.d("KnowledgebaseListFragment", "END initializing KnowledgebaseListFragment");
    }

    /**
     * create the knowledgebase-web-view
     *
     * @param inflater           to load web-view-layout
     * @param container          to load web-view-layout
     * @param savedInstanceState unused
     * @return created web-view
     */
    @Override
    public View onCreateView(@NonNull LayoutInflater inflater, ViewGroup container, Bundle savedInstanceState) {

        Log.d("KnowledgebaseListFragment", "START creating KnowledgebaseListView");
        
        //create web-view
        View view = inflater.inflate(R.layout.fragment_knowledgebaselist_list, container, false);

        // Set the adapter
        if (view instanceof RecyclerView) {
            Context context = view.getContext();
            RecyclerView recyclerView = (RecyclerView) view;
            recyclerView.setLayoutManager(new LinearLayoutManager(context));
            recyclerView.setAdapter(this.knowledgebaseListContent);
        }

        Log.d("KnowledgebaseListFragment", "END creating KnowledgebaseListView");
        
        //return created web-view
        return view;
    }

    /**
     * on click listener for knowledgebase-list
     */
    public interface OnListFragmentInteractionListener {

        /**
         * click on knowledgebase-list-item
         *
         * @param item selected knowledgebase-item
         */
        void openKnowledgebaseContentFragment(KnowledgebaseContentItem item);
    }
}
