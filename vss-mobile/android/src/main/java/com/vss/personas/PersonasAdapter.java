package com.vss.personas;


import android.view.LayoutInflater;
import android.view.View;
import android.view.ViewGroup;
import android.widget.TextView;

import androidx.recyclerview.widget.RecyclerView;

import com.vss.R;

public class PersonasAdapter extends RecyclerView.Adapter<PersonasAdapter.ViewHolder> {

    private final Persona[] personas;
    private final PersonasDelegate delegate;

    public PersonasAdapter(Persona[] personas, PersonasDelegate delegate) {
        this.personas = personas;
        this.delegate = delegate;
    }

    /**
     * Create new views (invoked by the layout manager).
     */

    @Override
    public ViewHolder onCreateViewHolder(ViewGroup viewGroup, int viewType) {
        // Create a new view, which defines the UI of the list item
        View view = LayoutInflater.from(viewGroup.getContext()).inflate(R.layout.persona_item, viewGroup, false);
        return new ViewHolder(view);
    }

    /**
     * Replace the contents of a view (invoked by the layout manager).
     */

    @Override
    public void onBindViewHolder(ViewHolder viewHolder, final int position) {
        viewHolder.getTextView().setText(personas[position].text);
        viewHolder.getView().setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View view) {
                delegate.onSelected(personas[position]);
            }
        });
    }

    @Override
    public int getItemCount() {
        return personas.length;
    }

    public interface PersonasDelegate {
        void onSelected(Persona persona);
    }

    public class ViewHolder extends RecyclerView.ViewHolder {
        private final TextView textView;
        private final View view;

        public ViewHolder(View view) {
            super(view);
            this.view = view;
            this.textView = (TextView) view.findViewById(R.id.name);
        }

        public TextView getTextView() {
            return textView;
        }

        public View getView() {
            return view;
        }
    }
}
