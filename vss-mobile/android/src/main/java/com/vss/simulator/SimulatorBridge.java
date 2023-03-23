package com.vss.simulator;

import android.content.res.AssetManager;
import android.util.Log;
import android.view.Surface;

/**
 * Bridge for accessing the simulator's native library.
 */
public class SimulatorBridge {
    private static final String LOG_TAG = "SimulatorBridge";
    private static boolean LIBRARY_LOADED = false;

    static {
        try {
            Log.d(LOG_TAG, "Loading native library...");
            System.loadLibrary("vss");
            LIBRARY_LOADED = true;
            Log.i(LOG_TAG, "Loading native library: successful");
        } catch (java.lang.UnsatisfiedLinkError e) {
            Log.e(LOG_TAG, "Loading native library: failed", e);
        }
    }

    private static native void nativeCreate(Surface surface, AssetManager assetManager);

    private static native void nativeDestroy();

    private static native void nativeResize(int width, int height);

    private static native void nativeDraw();

    private static native void nativePostFrame(int width, int height, byte[] y, byte[] u, byte[] v);

    private static native void nativePostSettings(String jsonString);

    public static boolean hasLoadedLibrary() {
        return LIBRARY_LOADED;
    }

    public static void create(Surface surface, AssetManager assetManager) {
        assert LIBRARY_LOADED : "Native library not loaded";
        Log.d(LOG_TAG, "Creating simulator");
        nativeCreate(surface, assetManager);
    }

    public static void destroy() {
        assert LIBRARY_LOADED : "Native library not loaded";
        Log.d(LOG_TAG, "Destroying simulator");
        nativeDestroy();
    }

    public static void resize(int width, int height) {
        assert LIBRARY_LOADED : "Native library not loaded";
        Log.v(LOG_TAG, "Resizing simulation to " + width + "x" + height);
        nativeResize(width, height);
    }

    public static void draw() {
        assert LIBRARY_LOADED : "Native library not loaded";
        Log.v(LOG_TAG, "Drawing simulation");
        nativeDraw();
    }

    public static void postFrame(int width, int height, byte[] y, byte[] u, byte[] v) {
        assert LIBRARY_LOADED : "Native library not loaded";
        Log.v(LOG_TAG, "Posting input frame");
        nativePostFrame(width, height, y, u, v);
    }

    public static void postSettings(String jsonString) {
        assert LIBRARY_LOADED : "Native library not loaded";
        Log.v(LOG_TAG, "Posting simulator settings: " + jsonString);
        nativePostSettings(jsonString);
    }
}
