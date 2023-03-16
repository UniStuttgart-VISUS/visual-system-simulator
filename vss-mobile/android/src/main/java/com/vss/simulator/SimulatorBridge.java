package com.vss.simulator;

import android.content.res.AssetManager;
import android.util.Log;
import android.view.Surface;

/**
 * Bridge for accessing the simulator's native library.
 */
public class SimulatorBridge {
    private static final String LOG_TAG = "SimulatorBridge";
    static private final long[] lastLogDraw = {0};
    static private final long[] lastLogPostFrame = {0};
    static private boolean loadedLibrary = false;

    static {
        try {
            Log.d(LOG_TAG, "Loading native library...");
            System.loadLibrary("vss");
            loadedLibrary = true;
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
        return loadedLibrary;
    }

    private static void throttleLog(long[] lastLog, String message) {
        if (lastLog[0] < System.currentTimeMillis() - 1000) {
            Log.v(LOG_TAG, message + " (throttled message)");
            lastLog[0] = System.currentTimeMillis();
        }
    }

    public static void create(Surface surface, AssetManager assetManager) {
        assert loadedLibrary : "Native library not loaded";
        Log.d(LOG_TAG, "Creating simulator");
        nativeCreate(surface, assetManager);
    }

    public static void destroy() {
        assert loadedLibrary : "Native library not loaded";
        Log.d(LOG_TAG, "Destroying simulator");
        nativeDestroy();
    }

    public static void resize(int width, int height) {
        assert loadedLibrary : "Native library not loaded";
        Log.d(LOG_TAG, "Resizing simulation to " + width + "x" + height);
        nativeResize(width, height);
    }

    public static void draw() {
        assert loadedLibrary : "Native library not loaded";
        throttleLog(lastLogDraw, "Drawing simulation");
        nativeDraw();
    }

    public static void postFrame(int width, int height, byte[] y, byte[] u, byte[] v) {
        assert loadedLibrary : "Native library not loaded";
        throttleLog(lastLogPostFrame, "Changing input frame to " + width + "x" + height);
        nativePostFrame(width, height, y, u, v);
    }

    public static void postSettings(String jsonString) {
        assert loadedLibrary : "Native library not loaded";
        Log.d(LOG_TAG, "Changing simulator settings to " + jsonString);
        nativePostSettings(jsonString);
    }
}
