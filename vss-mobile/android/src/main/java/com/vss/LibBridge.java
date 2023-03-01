package com.vss;

import android.content.res.AssetManager;
import android.util.Log;
import android.view.Surface;

/**
 * Bridge for accessing the native library.
 */
public class LibBridge {
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

    public static boolean hasLoadedLibrary() {
        return loadedLibrary;
    }

    public static void create(Surface surface, AssetManager assetManager) {
        assert loadedLibrary : "Native library not loaded";
        nativeCreate(surface, assetManager);
    }

    public static void destroy() {
        assert loadedLibrary : "Native library not loaded";
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
}
