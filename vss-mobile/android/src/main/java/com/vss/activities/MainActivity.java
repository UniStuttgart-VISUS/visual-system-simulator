package com.vss.activities;

import android.Manifest;
import android.app.AlertDialog;
import android.content.pm.PackageManager;
import android.os.Bundle;
import android.os.StrictMode;
import androidx.annotation.NonNull;
import androidx.core.app.ActivityCompat;
import androidx.appcompat.app.AppCompatActivity;
import androidx.recyclerview.widget.LinearLayoutManager;
import androidx.recyclerview.widget.RecyclerView;

import android.util.Log;
import android.view.LayoutInflater;
import android.view.Menu;
import android.view.MenuItem;
import android.view.View;
import android.view.ViewGroup;
import android.webkit.JavascriptInterface;
import android.webkit.WebView;
import android.widget.EditText;
import android.widget.Switch;
import android.widget.TextView;
import android.widget.Toast;

import java.io.IOException;

import com.vss.R;
import com.vss.fragments.SimulatorValueFragment;
import com.vss.fragments.KnowledgebaseListFragment;
import com.vss.fragments.ReadMeFragment;
import com.vss.fragments.SimulatorFragment;


/**
 * Main activity.
 */
public class MainActivity extends AppCompatActivity
        implements     ActivityCompat.OnRequestPermissionsResultCallback  {

    private static final int CAMERA_REQUEST_CODE = 100;

    private final String SPLIT_SCREEN_SWITCH_JSON_ID = "split_screen_switch";
    //private final String SENSOR_ORIENTATION = "sensor_orientation";


    private KnowledgebaseListFragment knowledgebaseListFragment;
    private SimulatorFragment welcomeFragment;
    private ReadMeFragment readmeFragment;

    private Menu navMenu;
    private Menu optMenu;


    private FragmentState fragmentState;

    private PersonasAdapter  personasAdapter  ;
    private  RecyclerView personasView ;

    private WebView inspectorView;

    private enum FragmentState {
        WELCOME, ED_SETTINGS, KNOWLEDGEBASE_LIST, KNOWLEDGEBASE_CONTENT, WEBUI_SETTINGS, READ_ME
    }


    /**
     * Android activity lifecycle: start.
     */
    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        setContentView(R.layout.activity_main);

       StrictMode.VmPolicy.Builder builder = new StrictMode.VmPolicy.Builder();
        StrictMode.setVmPolicy(builder.build());

        setContentView(R.layout.activity_main);

        //TODO: defer this to "as late as possible".
        checkPermissions();

        setupPersonasView();
        setupInspectorView();
        //setupSimulatorView();
    }

    private void checkPermissions() {
        if (ActivityCompat.checkSelfPermission(this, Manifest.permission.CAMERA) == PackageManager.PERMISSION_DENIED) {
            ActivityCompat.requestPermissions(this, new String[]{Manifest.permission.CAMERA}, CAMERA_REQUEST_CODE);
        }
    }

    private void setupPersonasView() {
        Persona[] personas =   {new Persona("yyy", "Custom", "<html>"),
                new Persona("xxx", "Achromataposie", "<html>"),
                new Persona("xxx", "Ametropie", "<html>"),
                new Persona("xxx", "Katarrakt", "<html>"),
                new Persona("xxx", "Dyschro...", "<html>"),
                new Persona("xxx", "Glaukom", "<html>"),
                new Persona("xxx", "X", "<html>"),
                new Persona("xxx", "Y", "<html>"),
                new Persona("xxx", "Z", "<html>")};
        personasAdapter = new PersonasAdapter(personas

        );

        personasView=     (RecyclerView) findViewById(R.id.preset_view) ;
        personasView.setHasFixedSize(true);
        LinearLayoutManager layoutManager = new LinearLayoutManager(this);
        layoutManager.setOrientation(LinearLayoutManager.HORIZONTAL);
        personasView.setLayoutManager(layoutManager);
        personasView.setAdapter(personasAdapter);



    }

    private class Persona {
        public String icon;
        public String name;
        public String details;

        public Persona(String icon, String name, String details) {
            this.icon = icon;
            this.name = name;
            this.details = details;
        }
    }

    private class PersonasAdapter extends RecyclerView.Adapter<PersonasAdapter.ViewHolder> {

        private Persona[] personas;


        /**
         * Provide a reference to the type of views that you are using
         * (custom ViewHolder)
         */
        public   class ViewHolder extends RecyclerView.ViewHolder {
            private final TextView textView;

            public ViewHolder(View view) {
                super(view);
                this.textView = (TextView) view.findViewById(R.id.name);
            }

            public TextView getTextView() {
                return textView;
            }
        }


        public PersonasAdapter(Persona[] personas) {
            this.personas = personas;
        }

        // Create new views (invoked by the layout manager)
        @Override
        public ViewHolder onCreateViewHolder(ViewGroup viewGroup, int viewType) {
            // Create a new view, which defines the UI of the list item
            View view = LayoutInflater.from(viewGroup.getContext())                    .inflate(R.layout.persona_item, viewGroup, false);
            return new ViewHolder(view);
        }

        // Replace the contents of a view (invoked by the layout manager)
        @Override
        public void onBindViewHolder(ViewHolder viewHolder, final int position) {
            viewHolder.getTextView().setText(personas[position].name);
        }

        @Override
        public int getItemCount() {
            return personas.length;
        }
    }

    @JavascriptInterface
    public void updateSimulator() {
        Toast.makeText(this, "message", Toast.LENGTH_SHORT).show();

    }

    private void setupInspectorView() {
        inspectorView=     (WebView) findViewById(R.id.inspector_view) ;


       // inspectorView.loadData("TODO", "text/html", "UTF-8");
        inspectorView.loadUrl("file:///android_asset/test.html");
        inspectorView.addJavascriptInterface(this, "Simulator");
    }

    /**
     * sets action for the backPress-button
     */
    @Override
    public void onBackPressed() {

       /* if (((DrawerLayout) findViewById(R.id.drawer_layout)).isDrawerOpen(GravityCompat.START)) {
            closeNavMenu();
        } else {

            switch (fragmentState) {
                case READ_ME:
                case WEBUI_SETTINGS:
                case KNOWLEDGEBASE_LIST:
                    openEDSettingsFragment();
                    break;
                case KNOWLEDGEBASE_CONTENT:
                    openKnowledgebaseListFragment();
                    break;
                case ED_SETTINGS:
                default:
                    super.onBackPressed();
                    break;
            }
        }

        */
    }

/*
END Android-Lifecycle
*/



/*
START Navigation Menu
 */

    /**
     * create navigation menu
     */
    private void onCreateNavigationMenu() {

//        Log.d("MainMenu", "START initializing navigation-menu");
//
//        //get view
//      //  NavigationView navigationView = findViewById(R.id.nav_view);
//
//        //navigation menu
//       // navMenu = navigationView.getMenu();
//
//
//        //initialize splitScreen-switch
//     //   Switch switchSplitScreenSettings = navMenu.findItem(R.id.nav_settings).getSubMenu().findItem(R.id.nav_splitscreen_simulation).getActionView().findViewById(R.id.nav_splitscreen_simulation_switch);
//     //   switchSplitScreenSettings.setOnClickListener(view -> setSplitScreenSimulation(switchSplitScreenSettings));
//
//
//        //web ui menu
//        //initMenuWebUI();
//
//        //remove knowledgebase menu-item if knowledgebase not exist
//        if (knowledgebaseListFragment == null) {
//            //navigationView.getMenu().findItem(R.id.nav_knowledgebase_overview).setVisible(false);
//        }
//
//        //set on click listener
//      //  navigationView.setNavigationItemSelectedListener(item -> {
///*
//            //switch the menu items
//            switch (item.getItemId()) {
//                case R.id.nav_start_simulation:
//                    startSimulation();
//                    closeNavMenu();
//                    break;
//                case R.id.nav_eyediseases_settings:
//                    openEDSettingsFragment();
//                    closeNavMenu();
//                    break;
//                case R.id.nav_knowledgebase_overview:
//                    openKnowledgebaseListFragment();
//                    closeNavMenu();
//                    break;
//                case R.id.nav_eyediseases_settings_reset:
//                    openResetSimulationSettingsDialog();
//                    closeNavMenu();
//                    break;
//                case R.id.nav_splitscreen_simulation:
//                    switchSplitScreenSettings.setChecked(!switchSplitScreenSettings.isChecked());
//                    setSplitScreenSimulation(switchSplitScreenSettings);
//                    break;
//                case R.id.nav_webui_onoff:
//                    //setWebUIState();
//                    break;
//                case R.id.nav_webui_server:
//                   // openServerDialog();
//                    break;
//                case R.id.nav_webui_channel:
//                   // openChannelDialog();
//                    break;
//                case R.id.nav_readme:
//                    openReadMeFragment();
//                    closeNavMenu();
//                    break;
//                case R.id.nav_app_feedback:
//                    //openFeedbackDialog();
//                    break;
//                default:
//                    defaultNavOptSelection();
//                    break;
//            }
//
//            return true;
//        });*/
//
//        Log.d("MainMenu", "START initializing navigation-menu");
    }

    private void openReadMeFragment() {
//        Log.d("MainMenu", "START open readme fragment");
//
//        //replace fragment
//        getSupportFragmentManager().beginTransaction().replace(R.id.main_fragment, this.readmeFragment).commit();
//
//        //set back-press mode
//        fragmentState = FragmentState.READ_ME;
//
//        Log.d("MainMenu", "START open readme fragment");
    }

    /**
     * open welcome fragment
     */
    private void openWelcomeFragment() {

        Log.d("MainMenu", "START open welcome fragment");

        //replace fragment
        getSupportFragmentManager().beginTransaction().replace(R.id.main_fragment, this.welcomeFragment).commit();

        //set back-press mode
        fragmentState = FragmentState.WELCOME;

        Log.d("MainMenu", "START open welcome fragment");
    }

    /**
     * open knowledgebase fragment
     */
    private void openKnowledgebaseListFragment() {

        Log.d("MainMenu", "START open knowledgebase list fragment");

        //replace fragment
        getSupportFragmentManager().beginTransaction().replace(R.id.main_fragment, this.knowledgebaseListFragment).commit();

        //set back-press mode
        fragmentState = FragmentState.KNOWLEDGEBASE_LIST;

        Log.d("MainMenu", "END open knowledgebase list fragment");
    }

    /**
     * called on knowledgebase list entry click
     *
     * @param item selected knowledgebase-item
     */
   // @Override
 /*   public void openKnowledgebaseContentFragment(KnowledgebaseContentItem item) {

        Log.d("MainMenu", "START open knowledgebase content fragment (" + item.title + ")");

        //replace fragment
      //  getSupportFragmentManager().beginTransaction().replace(R.id.main_fragment, new KnowledgebaseWebFragment(this.edSettingsController, item)).commit();

        //set back-press mode
        fragmentState = FragmentState.KNOWLEDGEBASE_CONTENT;

        Log.d("MainMenu", "END open knowledgebase content fragment (" + item.title + ")");
    }*/

    /**
     * close navigation menu
     */
    private void closeNavMenu() {
     //   ((DrawerLayout) findViewById(R.id.drawer_layout)).closeDrawer(GravityCompat.START);
    }

/*
END Navigation Menu
 */



/*
START Option Menu
 */

    /**
     * create option-menu
     *
     * @param menu menu to create
     * @return successful created
     */
    @Override
    public boolean onCreateOptionsMenu(Menu menu) {

        Log.d("MainMenu", "START initializing options-menu");

        optMenu = menu;

        //inflate option-menu
        getMenuInflater().inflate(R.menu.opt_menu, menu);

        Log.d("MainMenu", "END initializing options-menu");

        return true;
    }

    /**
     * on option-item-click
     *
     * @param item selected item
     * @return successful action
     */
    @Override
    public boolean onOptionsItemSelected(MenuItem item) {

        //switches menu items
        switch (item.getItemId()) {
            case R.id.opt_eyediseases_settings_load:
                openLoadSimulationSettingsDialog();
                break;
            case R.id.opt_eyediseases_settings_store:
                openStoreSimulationSettingsDialog();
                break;
            case R.id.opt_eyediseases_settings_reset:
                openResetSimulationSettingsDialog();
                break;
            default:
                defaultNavOptSelection();
                break;
        }
        return super.onOptionsItemSelected(item);
    }

/*
END Option Menu
*/



/*
START Simulation Functions
*/

  //  private EDSettingsController edSettingsController;
    private SimulatorValueFragment edSettingsFragment;

    /**
     * initialize simulation-settings-controller
     */
    private void onCreateSimulationSettingsController() {
        try {
           // this.edSettingsController = new EDSettingsController(this);
        } catch (Exception e) {
            //TODO check ui files, important !!!
            Log.e("UI-ERROR", "Make sure that ed_control_androidapp.html and ed_control_default_values.json exist in assets/ui!");
            e.printStackTrace();
        }
    }

    /**
     * starts the native activity with the simulation core
     */
    private void startSimulation() {

        Log.d("MainMenu", "START start Simulation-Activity");

        //print message
        Toast.makeText(this, R.string.start_simulation, Toast.LENGTH_LONG).show();

        //start native activity
    //    startActivity(new Intent(getBaseContext(), CoreLoaderActivity.class));

        Log.d("MainMenu", "END start Simulation-Activity");
    }

    /**
     * open simulation-settings fragment
     */
    private void openEDSettingsFragment() {

        Log.d("MainMenu", "START open simulation settings fragment");

        //replace fragment
       // getSupportFragmentManager().beginTransaction().replace(R.id.main_fragment, this.edSettingsFragment).commit();

        //set back-press mode
        fragmentState = FragmentState.ED_SETTINGS;

        Log.d("MainMenu", "END open simulation settings fragment");
    }

    /**
     * dialog to load simulation settings
     */
    private void openLoadSimulationSettingsDialog() {

        Log.d("MainMenu", "OPEN load simulation-settings dialog");

        //load simulation-settings-files
       // File[] settingFiles = this.edSettingsController.getSimulationSettingFileList().toArray(new File[this.edSettingsController.getSimulationSettingFileList().size()]);

        //create file-name-array
        //CharSequence[] settingNames = new CharSequence[settingFiles.length];
       // for (int i = 0; i < settingFiles.length; i++) {
        //    settingNames[i] = settingFiles[i].getName();
        //}

        //create dialog builder
       // AlertDialog.Builder builder = new AlertDialog.Builder(this);

        //build dialog
      //  builder.setTitle(R.string.eyediseases_settings_load)
      //          .setItems(settingNames, (dialog, which) -> loadSimulationSettings(which))
      //          .setPositiveButton(R.string.eyediseases_settings_reset, (dialogInterface, i) -> openResetSimulationSettingsDialog())
       //         .setNegativeButton(R.string.cancel, null)
       //         .setOnDismissListener(dialogInterface -> Log.d("MainMenu", "CLOSE load simulation-settings dialog"));

        //show dialog
      //  builder.show();
    }

    /**
     * dialog to store simulation-settings
     */
    private void openStoreSimulationSettingsDialog() {

        Log.d("MainMenu", "OPEN store simulation-settings dialog");

        //input test
        EditText input = new EditText(this);
        input.setSingleLine();

        //create dialog builder
        AlertDialog.Builder builder = new AlertDialog.Builder(this);

        //build dialog
        builder.setTitle(R.string.eyediseases_settings_store)
                .setMessage(R.string.save_as)
                .setView(input)
                .setPositiveButton(R.string.store, (dialogInterface, i) -> storeSimulationSettings(input.getText().toString()))
                .setNegativeButton(R.string.cancel, null)
                .setOnDismissListener(dialogInterface -> Log.d("MainMenu", "CLOSE store simulation-settings dialog"));

        //show dialog
        builder.show();
    }

    /**
     * dialog to confirm reset simulation-settings
     */
    private void openResetSimulationSettingsDialog() {

        Log.d("MainMenu", "OPEN reset simulation-settings dialog");

        //create dialog builder
        AlertDialog.Builder builder = new AlertDialog.Builder(this);

        //build dialog
        builder.setTitle(R.string.eyediseases_settings_reset)
                .setMessage(R.string.eyediseases_settings_reset_confirmation)
                .setPositiveButton(R.string.eyediseases_settings_reset, (dialogInterface, i) -> resetSimulationSettings())
                .setNegativeButton(R.string.cancel, null)
                .setOnDismissListener(dialogInterface -> Log.d("MainMenu", "CLOSE reset simulation-settings dialog"));

        //show dialog
        builder.show();
    }

    /**
     * load simulation-settings
     *
     * @param id settings-id
     */
    private void loadSimulationSettings(int id) {

        Log.d("MainMenu", "Load simulation-settings ...");

        //try to load simulation-settings
     //   try {

            //load simulation-settings from id
           // this.edSettingsController.loadSimulationSettings(id);

            //print message
            Toast.makeText(this, R.string.eyediseases_settings_load_successful, Toast.LENGTH_SHORT).show();

            Log.d("MainMenu", "Load simulation-settings successful!");
       // }

        //loading failed
       // catch (IOException | IndexOutOfBoundsException e) {

            //print message
        //    Toast.makeText(this, R.string.eyediseases_settings_load_failed, Toast.LENGTH_SHORT).show();

         //   Log.d("MainMenu", "Load simulation-settings failed!", e);
        //}
    }

    /**
     * store simulation-settings
     *
     * @param name name of simulation-settings
     */
    private void storeSimulationSettings(String name) {

        Log.d("MainMenu", "Store simulation-settings ...");

        //try to store simulation-settings
      //  try {

            //store simulation-settings with name
         //   edSettingsController.storeSimulationSettings(name);

            //print message
            Toast.makeText(this, R.string.eyediseases_settings_store_successful, Toast.LENGTH_SHORT).show();

            Log.d("MainMenu", "Store simulation-settings successful!");
      //  }

        //storing failed
      //  catch (IOException e) {

            //print message
       //     Toast.makeText(this, R.string.eyediseases_settings_store_failed, Toast.LENGTH_SHORT).show();

       //     Log.d("MainMenu", "Store simulation-settings failed", e);
      //  }
    }

    /**
     * resets the simulation-settings
     */
    private void resetSimulationSettings() {

        Log.d("MainMenu", "Reset simulation-settings ...");

        //try to reset simulation-settings
      //  try {

            //load default simulation-settings
        //    edSettingsController.loadDefaultSimulationSettings();

            //print message
            Toast.makeText(this, R.string.eyediseases_settings_reset_successful, Toast.LENGTH_SHORT).show();

            Log.d("MainMenu", "Reset simulation-settings successful!");
       // }

        //reset failed
      //  catch (IOException e) {

            //print message
        //    Toast.makeText(this, R.string.eyediseases_settings_reset_failed, Toast.LENGTH_SHORT).show();

       //     Log.d("MainMenu", "Reset simulation-settings failed!", e);
      //  }
    }

    /**
     * store split-screen mode setting
     *
     * @param s split-screen-switch
     */
    private void setSplitScreenSimulation(Switch s) {

        Log.d("MainMenu", "START set split screen switch");

        //load switch state
        boolean checked = s.isChecked();

        //update settings with switch state
   //     edSettingsController.updateSettings(this, SPLIT_SCREEN_SWITCH_JSON_ID, Boolean.toString(checked));

        //print status-toast
        if (checked) {
            Toast.makeText(this, R.string.splitscreen_activated, Toast.LENGTH_SHORT).show();
        } else {
            Toast.makeText(this, R.string.splitscreen_deactivated, Toast.LENGTH_SHORT).show();
        }

        Log.d("MainMenu", "END set split screen switch");
    }

    /**
     * reload the split-screen-state from the settings-controller
     */
    private void reloadSplitScreenSwitch() {

        Log.d("MainMenu", "START reload split screen switch");

        //load switch
    //    Switch s = ((NavigationView) findViewById(R.id.nav_view)).getMenu().findItem(R.id.nav_settings).getSubMenu().findItem(R.id.nav_splitscreen_simulation).getActionView().findViewById(R.id.nav_splitscreen_simulation_switch);

        //load settings
        //s.setChecked(edSettingsController.getBooleanValue(SPLIT_SCREEN_SWITCH_JSON_ID));

        Log.d("MainMenu", "END reload split screen switch");
    }

/*
END Simulation Functions
*/



/*
START
*/

     @Override
    public void onRequestPermissionsResult(int requestCode, @NonNull String[] permissions, @NonNull int[] grantResults) {
       super.onRequestPermissionsResult(requestCode, permissions, grantResults);
        if (requestCode ==  CAMERA_REQUEST_CODE) {
            if (grantResults[0] == PackageManager.PERMISSION_GRANTED) {
                Log.w("Permission", "Camera: DENIED");
            } else {
                Log.i("Permission", "Camera: GRANTED");
            }
        }
    }
    private void defaultNavOptSelection() {
        Toast.makeText(this, "not implemented jet", Toast.LENGTH_SHORT).show();
    }

    //@Override
    public void updateSettings(String simulationSettings) throws IOException {
        runOnUiThread(this::reloadSplitScreenSwitch);
    }


/*
END
*/
}