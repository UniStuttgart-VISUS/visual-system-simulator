package de.uni_stuttgart.vss.edsettings;

import java.io.IOException;
import java.util.ArrayList;
import java.util.List;

/**
 * listener for simulation-settings
 */
public interface EDSettingsListener {

    /**
     * provides all listeners
     */
    class Provider {

        /**
         * provider singleton-instance
         */
        private static Provider provider;

        /**
         * get provider singleton-instance
         *
         * @return singleton-instance
         */
        public static Provider getInstance() {

            //singleton-instance not initialized
            if (provider == null) {

                //initialize instance
                provider = new Provider();
            }

            //return singleton-instance
            return provider;
        }

        /**
         * private constructor initializes listeners-list
         */
        private Provider() {
            listeners = new ArrayList<>();
        }

        /**
         * listeners list
         */
        private List<EDSettingsListener> listeners;

        /**
         * add simulation-settings-listener
         *
         * @param listener listener to add
         */
        public void addListener(EDSettingsListener listener) {

            //list is not containing listener already
            if (!listeners.contains(listener)) {

                //add listener
                listeners.add(listener);
            }
        }

        /**
         * get the listeners-list
         *
         * @return listeners-list
         */
        List<EDSettingsListener> getListeners() {
            return listeners;
        }
    }

    /**
     * called when simulation settings have changed
     *
     * @param simulationSettings new simulation-settings
     * @throws IOException could not update listener
     */
    void updateSettings(String simulationSettings) throws IOException;
}
