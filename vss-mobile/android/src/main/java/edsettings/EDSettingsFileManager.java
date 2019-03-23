package de.uni_stuttgart.vss.edsettings;

import android.annotation.SuppressLint;
import android.content.Context;
import android.util.Log;

import org.json.simple.JSONObject;
import org.json.simple.parser.JSONParser;
import org.json.simple.parser.ParseException;

import java.io.File;
import java.io.FileNotFoundException;
import java.io.FileReader;
import java.io.FileWriter;
import java.io.IOException;
import java.io.InputStreamReader;
import java.util.ArrayList;
import java.util.List;

/**
 * file-manager
 */
public class EDSettingsFileManager implements EDSettingsListener {

    /**
     * simulation-settings files directory
     */
    private File settingsFilesDirectory;
    private static final String SETTINGS_FILES_DIRECTORY_NAME = "settings_files";

    /**
     * temporary simulation-settings file
     */
    private File tempSettingsFile;
    private static final String TEMP_SETTINGS_FILE_NAME = "_temp_simulation_settings.json";

    /**
     * default simulation-settings
     */
    private String defaultSettings;
    private JSONObject defaultSettingsObject;

    /**
     * current simulation-settings
     */
    private File currentSettingsFile;

/*
START Initialization
*/

    /**
     * singleton-instance
     */
    private static EDSettingsFileManager edSettingsFileManager;

    /**
     * returns the singleton-instance
     *
     * @param c app-context to create instance
     * @return singleton-instance
     */
    static EDSettingsFileManager getInstance(Context c) throws Exception {

        //no instance exists
        if (edSettingsFileManager == null) {

            //initialize file-manager
            edSettingsFileManager = new EDSettingsFileManager(c);
        }

        //return instance
        return edSettingsFileManager;
    }

    /**
     * construct the file-manager
     * load settings-file-directory
     * load temporary-settings-file
     * load default-settings-file
     * initialize current-settings-file
     *
     * @param c app-context to create instance
     */
    private EDSettingsFileManager(Context c) throws Exception {

        Log.d("EDSettingsFileManager", "START initializing EDSettingsFileManager");

        //load directory for settings files
        this.settingsFilesDirectory = new File(c.getFilesDir().getAbsolutePath() + "/" + SETTINGS_FILES_DIRECTORY_NAME);

        //
        if (!settingsFilesDirectory.exists()) {
            if (settingsFilesDirectory.mkdir()) {
                Log.d("EDSettingsFileManager", "START initializing EDSettingsFileManager");
            } else {
                //todo
            }
        }

        //load temporary-settings-file
        this.tempSettingsFile = new File(this.settingsFilesDirectory.getAbsolutePath() + "/" + TEMP_SETTINGS_FILE_NAME);


        // load default-settings-file
        try {
            InputStreamReader isr = new InputStreamReader(c.getAssets().open("ui/ed_control_default_values.json"));
            this.defaultSettingsObject = (JSONObject) new JSONParser().parse(isr);
            this.defaultSettings = defaultSettingsObject.toJSONString();
        } catch (ParseException | IOException e) {
            throw new Exception("Default Simulation-Settings could not be load!");
        }


        //init current-settings-file
        try {
            this.initCurrentSettingsFile();
        } catch (IOException e) {
            e.printStackTrace();
        }

        Log.d("EDSettingsFileManager", "END initializing EDSettingsFileManager");
    }

    /**
     * initialize current-settings-file
     *
     * @throws IOException
     * @throws ParseException
     */
    private void initCurrentSettingsFile() throws IOException {

        Log.d("EDSettingsFileManager", "START initializing current file settings");

        //temp-settings-file exists
        if (tempSettingsFile != null && tempSettingsFile.exists()) {

            //load settings from temp-file
            this.loadSettingsFile(this.tempSettingsFile);
        }

        //temp-settings-file did not exists
        else {

            //create temp-settings-file successful
            if (tempSettingsFile.createNewFile()) {

                //load default-settings into temp-settings-file
                this.loadDefaultSettingsFile();
            }

            //create temp-settings-file failed
            else {

                //throw exception
                throw new FileNotFoundException("File(" + TEMP_SETTINGS_FILE_NAME + ") was not found and could not be created!");
            }
        }

        Log.d("EDSettingsFileManager", "END initializing current file settings");
    }

/*
END Initialization
*/



/*
START Read/Write Settings
*/

    /**
     * get json-object from current simulation-settings
     *
     * @return json-object containing current simulation-settings
     * @throws IOException loading current settings from file failed
     */
    JSONObject getCurrentSettings() {

        //try to return json object from current settings-file
        try {
            System.out.println("Pre parse");
            // read settings-file, parse to json-object, return object
            JSONObject j = (JSONObject) new JSONParser().parse(new FileReader(getCurrentSettingsFile()));
            EDSettingsController.updateSettingsForce(j);
            return j;
        }

        //load default-settings-file and return json-object
        catch (ParseException | IOException e) {

            //todo
            e.printStackTrace();

            //load default-settings
            try {
                this.loadDefaultSettingsFile();
            } catch (IOException e1) {
                e1.printStackTrace();
            }

            //return json-object
            EDSettingsController.updateSettingsForce(this.defaultSettingsObject);
            return this.defaultSettingsObject;
        }
    }

    /**
     * returns the json-settings-file and creates it from default, if it doesn't exist
     *
     * @return json-settings-file
     */
    private File getCurrentSettingsFile() throws IOException {

        //init current-settings-file if not exists
        if (currentSettingsFile == null || !currentSettingsFile.exists()) {
            initCurrentSettingsFile();
        }

        //return json-settings-file
        return currentSettingsFile;
    }

    /**
     * returns a list, containing all settings-files without the temp-settings-file
     *
     * @return list containing all settings-files
     */
    List<File> getAllSettingsFiles() {

        //list containing all settings files
        List<File> fileList = new ArrayList<>();

        //settings-file in directory
        File[] files = this.settingsFilesDirectory.listFiles();

        //add all file except the temp-settings-file to list
        for (File file : files) {
            if (!file.getName().equals(tempSettingsFile.getName())) {
                fileList.add(file);
            }
        }

        //return list
        return fileList;
    }

    /**
     * called from the eye diseases settings controller to update the values
     *
     * @param simulationSettings simulation-settings-string in json-format
     * @throws IOException could nor write updated settings to temp-file
     */
    @Override
    public void updateSettings(String simulationSettings) throws IOException {

        //current-settings-file is a stored file -> load writable temp-settings-file
        if (currentSettingsFile != tempSettingsFile) {
            loadSettingsFile(tempSettingsFile);
        }

        //write new values to temp-settings-file
        writeSettingsToFile(getCurrentSettingsFile(), simulationSettings);
    }

    /**
     * writes settings into the file
     *
     * @param file     to store in
     * @param settings to store
     * @throws IOException could not write settings to file
     */
    private void writeSettingsToFile(File file, String settings) throws IOException {
        FileWriter fileWriter = new FileWriter(file);
        fileWriter.write(settings);
        fileWriter.flush();
        fileWriter.close();
        Log.d(this.toString(), file.getName() + " written!");
    }

/*
END Read/Write Settings
*/



/*
START Store/Load Settings
*/

    /**
     * create a file with the settings-name and store the current settings
     *
     * @param settingsName file-name
     * @return store successfully, depending if filename exists already or not
     * @throws IOException could not write new settings to file
     */
    @SuppressLint("NewApi")
    boolean storeCurrentSettingsToFile(String settingsName) throws IOException {

        //new File to store the settings
        File settingsFile = new File(this.settingsFilesDirectory.getAbsolutePath() + "/" + settingsName);

        //file not exists already
        if (!settingsFile.exists()) {

            //create file
            settingsFile.createNewFile();
        }

        //file exists already
        else {
            return false;
        }

        //write settings to file
        this.writeSettingsToFile(settingsFile, getCurrentSettings().toJSONString());
        return true;
    }

    /**
     * load a simulation-settings-file
     *
     * @param file to load
     */
    void loadSettingsFile(File file) {

        //check if file is valid
        if (this.getAllSettingsFiles().contains(file) || file.equals(this.tempSettingsFile)) {

            //set the current-settings-file to the file
            this.currentSettingsFile = file;
        }
    }

    /**
     * load the default-settings-file
     *
     * @throws IOException could not write default settings to file
     */
    void loadDefaultSettingsFile() throws IOException {

        //print log
        Log.d("JSON-VALUES-FILE", "load default-json-values-file");

        //write default values to temp-values-file
        writeSettingsToFile(this.tempSettingsFile, this.defaultSettings);

        //set the current-simulated-values to the temp-values
        this.loadSettingsFile(this.tempSettingsFile);
    }

/*
END Store/Load Settings
*/
}
