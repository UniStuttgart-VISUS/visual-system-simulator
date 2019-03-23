package de.uni_stuttgart.vss;

import android.util.Log;

/**
 * entry in the knowledgebase
 */
public class KnowledgebaseContentItem {
    public final String id;
    public final String title;
    public final String contentUrl;

    /**
     * initialisation
     *
     * @param id          item id
     * @param title       item title
     * @param contentFile file in the assets
     */
    KnowledgebaseContentItem(String id, String title, String contentFile) {

        Log.d("KnowledgebaseItem", "START initializing KnowledgebaseItem (" + title + ")");

        //initialisation
        this.id = id;
        this.title = title;
        this.contentUrl = "file:///android_asset/" + contentFile;

        Log.d("KnowledgebaseItem", "START initializing KnowledgebaseItem (" + title + ")");
    }
}
