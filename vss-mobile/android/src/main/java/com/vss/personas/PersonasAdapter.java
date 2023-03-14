package com.vss.personas;


import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;

import androidx.recyclerview.widget.RecyclerView;

import com.vss.R;

public class PersonasAdapter extends RecyclerView.Adapter<PersonasAdapter.ViewHolder> {

    private Persona[] personas;

    public PersonasAdapter(Persona[] personas) {
        this.personas = personas;
    }

    /**
     * Create new views (invoked by the layout manager).
     */

    @Override
    public ViewHolder onCreateViewHolder(ViewGroup viewGroup, int viewType) {
        // Create a new view, which defines the UI of the list item
        View view = LayoutInflater.from(viewGroup.getContext()).inflate(R.layout.persona_item, viewGroup, false);
        view.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                //TODO
            }
        });
        return new ViewHolder(view);
    }

    /**
     * Replace the contents of a view (invoked by the layout manager).
     */

    @Override
    public void onBindViewHolder(ViewHolder viewHolder, final int position) {
        viewHolder.getTextView().setText(personas[position].text);
    }

    @Override
    public int getItemCount() {
        return personas.length;
    }

    public class ViewHolder extends RecyclerView.ViewHolder {
        private final TextView textView;

        public ViewHolder(View view) {
            super(view);
            this.textView = (TextView) view.findViewById(R.id.name);
        }

        public TextView getTextView() {
            return textView;
        }
    }
}
