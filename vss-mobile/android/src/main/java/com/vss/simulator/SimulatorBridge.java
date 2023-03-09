package com.vss.simulator;

import android.content.res.AssetManager;
import android.util.Log;
import android.view.Surface;
import android.webkit.JavascriptInterface;

/**
 * Bridge for accessing the simulator's native library.
 */
public class SimulatorBridge {
    static private boolean loadedLibrary = false;

    static {
        try {
            Log.d("NativeBridge", "Loading native library...");
            System.loadLibrary("vss");
            loadedLibrary = true;
            Log.i("NativeBridge", "Loading native library: successful");
        } catch (java.lang.UnsatisfiedLinkError e) {
            Log.e("NativeBridge", "Loading native library: failed", e);
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

    public static void create(Surface surface, AssetManager assetManager) {
        assert loadedLibrary : "Native library not loaded";
        Log.d("SimulatorBridge", "Creating...");
        nativeCreate(surface, assetManager);
    }

    public static void destroy() {
        assert loadedLibrary : "Native library not loaded";
        Log.d("SimulatorBridge", "Destroying...");
        nativeDestroy();
    }

    public static void resize(int width, int height) {
        assert loadedLibrary : "Native library not loaded";
        nativeResize(width, height);
    }

    public static void draw() {
        assert loadedLibrary : "Native library not loaded";
        nativeDraw();
    }

    public static void postFrame(int width, int height, byte[] y, byte[] u, byte[] v) {
        assert loadedLibrary : "Native library not loaded";
        nativePostFrame(width, height, y, u, v);
    }

    public static void postSettings(String jsonString) {
        assert loadedLibrary : "Native library not loaded";
        nativePostSettings(jsonString);
    }
}
