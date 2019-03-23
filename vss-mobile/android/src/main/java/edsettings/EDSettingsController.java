package de.uni_stuttgart.vss.edsettings;

import android.content.Context;
import android.util.Log;

import org.json.simple.JSONObject;

import java.io.File;
import java.io.IOException;
import java.util.List;
import java.util.concurrent.atomic.AtomicReference;


/**
 * controller for the simulation settings
 */
public class EDSettingsController {

    /**
     * file-manager for settings-files
     */
    private EDSettingsFileManager fileManager;

    /**
     * eye diseases setting in json-object
     */
    private JSONObject object;

    /**
     * eye diseases in json-formatted-string
     */
    private static AtomicReference<String> simulationSettings = new AtomicReference<>("UNINITIALIZED");



/*
START Initialization
*/

    /**
     * constructs the eye diseases settings controller
     * initializes listeners list
     * initializes the file-manager - load setting - add file-manager to listeners
     *
     * @param context to access file system
     */
    public EDSettingsController(Context context) throws Exception {

        Log.d("EDSettingsController", "START initializing EDSettingsController");

        //try to initialize file-manager

        //initialize file-manager
        fileManager = EDSettingsFileManager.getInstance(context);

        //load settings
        object = fileManager.getCurrentSettings();

        //add file-manager to listeners
        this.addListener(fileManager);

        Log.d("EDSettingsController", "END initializing EDSettingsController");
    }

    /**
     * adds a listener to the list of eye diseases settings listeners
     *
     * @param listener eye diseases settings changed listener
     */
    public void addListener(EDSettingsListener listener) {

        Log.d("EDSettingsController", "START adding EDSettingsListener");

        //add settings-listener to list
        EDSettingsListener.Provider.getInstance().addListener(listener);

        //try to update listener
        try {

            //update listener with current settings
            listener.updateSettings(getSimulationSettings());
        }

        //failed to update listener
        catch (IOException e) {

            Log.d("EDSettingsController", "Update listener (" + listener.toString() + ") failed!", e);
        }

        Log.d("EDSettingsController", "END adding EDSettingsListener");
    }

/*
END Initialization
*/



/*
START Read/Write Settings
*/

    /**
     * activate eye diseases with the id
     *
     * @param caller instance called the function
     * @param id     eye disease to activate
     */
    public void activateED(EDSettingsListener caller, String id) {
        updateSettings(caller, id, "true");
    }

    /**
     * deactivate eye diseases with the id
     *
     * @param caller instance called the function
     * @param id     eye disease to deactivate
     */
    public void deactivateED(EDSettingsListener caller, String id) {
        updateSettings(caller, id, "false");
    }

    /**
     * update setting from id to value
     *
     * @param caller instance called the function
     * @param id     to update
     * @param value  to update
     */
    public void updateSettings(EDSettingsListener caller, String id, String value) {

        new Thread(() -> {

            Log.d("EDSettingsController", "START update settings (" + id + ") to (" + value + ")");

            //change value only if id exists in json-file
            if (object.containsKey(id)) {

                //get the old value
                Object oldValue = object.get(id);

                //compare type of old value to find type of the new value
                if (oldValue instanceof Boolean) object.put(id, Boolean.valueOf(value));
                else if (oldValue instanceof Long) object.put(id, Long.valueOf(value));
                else if (oldValue instanceof String) object.put(id, value);

                //update listeners
                this.updateListeners(caller);
            }

            Log.d("EDSettingsController", "END update settings (" + id + ") to (" + value + ")");

        }).start();
    }

    /**
     * update all setting
     *
     * @param caller   instance called the function
     * @param settings to update
     */
    public void updateSettings(EDSettingsListener caller, JSONObject settings) {

        new Thread(() -> {


            Log.d("EDSettingsController", "START update all settings");

            //parse String into JSON-Object
            this.object = settings;

            //update listeners
            this.updateListeners(caller);

            Log.d("EDSettingsController", "END update all settings");

        }).start();
    }

    public static void updateSettingsForce(JSONObject settings){
        simulationSettings.set(settings.toJSONString());
    }

    /**
     * update settings and notify listeners except the caller-listener
     *
     * @param caller listener will not be notified
     */
    private void updateListeners(EDSettingsListener caller) {

        Log.d("EDSettingsController", "START update listeners");

        //write it in global variable for NativeActivity
        simulationSettings.set(this.object.toJSONString());

        //notify listeners
        EDSettingsListener.Provider.getInstance().getListeners().forEach((listener) -> {

            //listener is not the one who called the update-settings methode
            if (listener != caller) {

                //try to update listener
                try {

                    //update listener with simulation-settings
                    listener.updateSettings(getSimulationSettings());
                }

                //update listener failed
                catch (IOException e) {

                    Log.d("EDSettingsController", "Update listener (" + listener.toString() + ") failed!", e);
                }
            }
        });

        Log.d("EDSettingsController", "END update listeners");
    }

    /**
     * return long-value from settings-id
     *
     * @param id setting-id
     * @return value from id
     */
    public long getLongValue(String id) {

        //no json-file found - default 0
        if (this.object == null) {
            return 0;
        }

        //get, cast, print and return value
        return (long) this.object.get(id);
    }

    /**
     * return boolean-value from settings-id
     *
     * @param id setting-id
     * @return value from id
     */
    public boolean getBooleanValue(String id) {

        //no json file-file found - default false
        if (this.object == null) {
            return false;
        }

        //get, cast, print and return value
        return (boolean) this.object.get(id);
    }

    /**
     * return string-value from settings-id
     *
     * @param id setting-id
     * @return value from id
     */
    public String getStringValue(String id) {

        //no json file-file found - default false
        if (this.object == null) {
            return "";
        }

        //get, cast, print and return value
        return (String) this.object.get(id);
    }

    /**
     * returns the simulation-settings as text in json-formatted-string
     *
     * @return simulation-settings as json-formatted-string
     */
    public static String getSimulationSettings() {

        //convert atomic into string
        return simulationSettings.get();
    }

/*
END Read/Write Settings
*/



/*
START Store/Load Settings
*/

    /**
     * store simulation-settings
     *
     * @param settingsName simulation-settings-name
     * @throws IOException failed to store settings into file
     */
    public void storeSimulationSettings(String settingsName) throws IOException {

        Log.d("EDSettingsController", "START store simulation-settings");

        //store simulation-settings into file
        fileManager.storeCurrentSettingsToFile(settingsName);

        Log.d("EDSettingsController", "END store simulation-settings");
    }

    /**
     * load simulation-settings form file
     *
     * @param file from list of all simulation settings
     * @throws IOException failed to load settings from file
     */
    private void loadSimulationSettings(File file) throws IOException {

        Log.d("EDSettingsController", "START load simulation-settings");

        //load settings into file manager
        fileManager.loadSettingsFile(file);

        //load settings from file-manager into controller
        this.object = this.fileManager.getCurrentSettings();

        //notify listeners that settings have changed
        this.updateListeners(fileManager);

        Log.d("EDSettingsController", "END load simulation-settings");
    }

    /**
     * load settings from file index
     *
     * @param index index in file-list of the file to load
     * @throws IOException               failed to load settings from file
     * @throws IndexOutOfBoundsException file-index out of bounds
     */
    public void loadSimulationSettings(int index) throws IOException, IndexOutOfBoundsException {

        //check if index valid
        if (index >= getSimulationSettingFileList().size() || index < 0) {

            //index out of bounds
            throw new IndexOutOfBoundsException("Simulation-settings-file index out of bounds");
        }

        //load simulation-settings
        loadSimulationSettings(getSimulationSettingFileList().get(index));
    }

    /**
     * load default simulation-settings
     *
     * @throws IOException failed to load default-simulation-settings
     */
    public void loadDefaultSimulationSettings() throws IOException {

        Log.d("EDSettingsController", "START load default-simulation-settings");

        //load default settings into file manager
        this.fileManager.loadDefaultSettingsFile();

        //load settings from file-manager into controller
        this.object = this.fileManager.getCurrentSettings();

        //notify listeners that settings have changed
        this.updateListeners(fileManager);

        Log.d("EDSettingsController", "END load default-simulation-settings");
    }

    /**
     * load prepared settings from knowledgebase preset
     *
     * @param file settings-file-name
     */
    public void loadPreparedSimulationSettings(String file) throws IOException {
        //TODO load predefined sim-settings from knowledgebase
        loadSimulationSettings(new File(file));
    }

    /**
     * returns a list of valid simulation-settings-files
     *
     * @return list of simulation-settings-files
     */
    public List<File> getSimulationSettingFileList() {
        return fileManager.getAllSettingsFiles();
    }

/*
END Store/Load Settings
*/
}
