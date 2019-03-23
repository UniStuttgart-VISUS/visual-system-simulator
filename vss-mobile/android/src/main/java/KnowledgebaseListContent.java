package de.uni_stuttgart.vss;

import android.content.Context;
import android.support.v7.widget.RecyclerView;
import android.util.Log;
import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;

import de.uni_stuttgart.vss.fragments.KnowledgebaseListFragment.OnListFragmentInteractionListener;


public class KnowledgebaseListContent extends RecyclerView.Adapter<KnowledgebaseListContent.ViewHolder> {

    /**
     * directory in the assets containing the knowledgebase
     */
    private static final String KNOWLEDGE_BASE_DIRECTORY = "knowledgebase";

    /**
     * list of knowledgebase-items
     */
    private final List<KnowledgebaseContentItem> mValues;

    /**
     * on item select from list
     */
    private final OnListFragmentInteractionListener mListener;

    /**
     * initialize knowledgebase-list and add listener
     *
     * @param listener listener for item select from list
     * @param context  context to load the knowledgebase-files from the assets
     */
    public KnowledgebaseListContent(OnListFragmentInteractionListener listener, Context context) throws IOException {

        Log.d("KnowledgebaseListContent", "START initializing KnowledgebaseListContent");

        mValues = getContent(context);
        mListener = listener;

        Log.d("KnowledgebaseListContent", "END initializing KnowledgebaseListContent");
    }

    /**
     * create the knowledgebase-list-view
     *
     * @param parent   to load view-layout
     * @param viewType to load view-layout
     * @return crated view
     */
    @Override
    public ViewHolder onCreateViewHolder(ViewGroup parent, int viewType) {

        Log.d("KnowledgebaseListContent", "START creating KnowledgebaseListContentView");

        //create view
        View view = LayoutInflater.from(parent.getContext()).inflate(R.layout.fragment_knowledgebaselist, parent, false);
        ViewHolder holder = new ViewHolder(view);

        Log.d("KnowledgebaseListContent", "END creating KnowledgebaseListContentView");

        //return crated view
        return holder;
    }

    /**
     * item is selected
     *
     * @param holder   view holder
     * @param position position of the selected item
     */
    @Override
    public void onBindViewHolder(final ViewHolder holder, int position) {

        //load selected item
        holder.mItem = mValues.get(position);
        holder.mIdView.setText(mValues.get(position).id);
        holder.mContentView.setText(mValues.get(position).title);

        //action to do on click
        holder.mView.setOnClickListener(v -> {
            if (null != mListener) {
                mListener.openKnowledgebaseContentFragment(holder.mItem);
            }
        });
    }

    /**
     * count knowledgebase-items
     *
     * @return count
     */
    @Override
    public int getItemCount() {
        return mValues.size();
    }

    /**
     * returns the content for the knowledgebase-list
     *
     * @param context app context
     * @return list of knowledgebase-elements
     */
    private List<KnowledgebaseContentItem> getContent(Context context) throws IOException {

        Log.d("KnowledgebaseListContent", "START load knowledgebase-items");

        //list to store knowledgebase-items
        List<KnowledgebaseContentItem> contentItemList = new ArrayList<>();

        //for each file in assets/knowledgebase
        String[] fileList = context.getAssets().list(KNOWLEDGE_BASE_DIRECTORY);
        int ctr = 0;
        for (int id = 0; id < fileList.length; id++) {
            if(!fileList[id].endsWith("html")){
                continue;
            }
            if(Locale.getDefault().getDisplayLanguage().contains("English")&&!fileList[id].contains("_en")){
                continue;
            }
            else if(Locale.getDefault().getDisplayLanguage().contains("Deutsch")&&fileList[id].contains("_en")){
                continue;
            }

            //file-path
            String contentFile = KNOWLEDGE_BASE_DIRECTORY + "/" + fileList[id];

            //input from file
            InputStreamReader isr = new InputStreamReader(context.getAssets().open(contentFile));
            BufferedReader br = new BufferedReader(isr);

            //file-string
            final String[] file = {""};
            br.lines().forEach(line -> file[0] = file[0] + line);

            //title of html-file
            if (file[0].contains("<title>") && file[0].contains("</title>")) {

                //get title from html-file
                String title = file[0].substring(file[0].indexOf("<title>") + 7, file[0].indexOf("</title>"));

                //create content file and add it to the list
                contentItemList.add(new KnowledgebaseContentItem(Integer.toString(ctr + 1) + ".", title, contentFile));
            }
            ctr++;
        }

        Log.d("KnowledgebaseListContent", "END load knowledgebase-items");

        //return list
        return contentItemList;
    }

    /**
     * view-holder containing the view
     */
    public class ViewHolder extends RecyclerView.ViewHolder {
        final View mView;
        final TextView mIdView;
        final TextView mContentView;
        KnowledgebaseContentItem mItem;

        /**
         * constructor
         *
         * @param view knowledgebase-list-view
         */
        ViewHolder(View view) {
            super(view);
            mView = view;
            mIdView = view.findViewById(R.id.id);
            mContentView = view.findViewById(R.id.content);
        }

        /**
         * get the string value from an knowledgebase-item
         *
         * @return string from knowledge-base-item
         */
        @Override
        public String toString() {
            return super.toString() + " '" + mContentView.getText() + "'";
        }
    }


}
