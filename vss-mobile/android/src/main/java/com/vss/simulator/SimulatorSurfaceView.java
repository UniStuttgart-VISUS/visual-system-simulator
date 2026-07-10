package com.vss.simulator;

import android.content.Context;
import android.hardware.HardwareBuffer;
import android.os.Looper;
import android.util.AttributeSet;
import android.view.SurfaceHolder;
import android.view.SurfaceView;

import androidx.annotation.NonNull;

/**
 * Surface view for simulator rendering.
 */
public class SimulatorSurfaceView extends SurfaceView implements SurfaceHolder.Callback2 {

    public SimulatorSurfaceView(Context context) {
        super(context);
        initialize();
    }

    public SimulatorSurfaceView(Context context, AttributeSet attrs) {
        super(context, attrs);
        initialize();
    }

    public SimulatorSurfaceView(Context context, AttributeSet attrs, int defStyleAttr) {
        super(context, attrs, defStyleAttr);
        initialize();
    }

    private void initialize() {
        this.setAlpha(1.0f);
        this.getHolder().addCallback(this);
    }

    @Override
    public void surfaceCreated(@NonNull SurfaceHolder holder) {
        assert Looper.getMainLooper().isCurrentThread() : "Called from non-UI thread";
        SimulatorBridge.create(holder.getSurface(), getResources().getAssets());
    }

    @Override
    public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
        assert Looper.getMainLooper().isCurrentThread() : "Called from non-UI thread";
        SimulatorBridge.resize(width, height);
    }

    @Override
    public void surfaceDestroyed(@NonNull SurfaceHolder holder) {
        assert Looper.getMainLooper().isCurrentThread() : "Called from non-UI thread";
        SimulatorBridge.destroy();
    }

    @Override
    public void surfaceRedrawNeeded(SurfaceHolder holder) {
        assert Looper.getMainLooper().isCurrentThread() : "Called from non-UI thread";
        SimulatorBridge.draw();
    }

    public void postHardwareBuffer(
            int width,
            int height,
            int dataSpace,
            int rotationDegrees,
            HardwareBuffer hardwareBuffer
    ) {
        assert Looper.getMainLooper().isCurrentThread() : "Called from non-UI thread";
        SimulatorBridge.postHardwareBuffer(width, height, dataSpace, rotationDegrees, hardwareBuffer);
        SimulatorBridge.draw();
    }

    public void postRgba(int width, int height, java.nio.ByteBuffer pixels) {
        assert Looper.getMainLooper().isCurrentThread() : "Called from non-UI thread";
        SimulatorBridge.postRgba(width, height, pixels);
        SimulatorBridge.draw();
    }

    public void postSettings(String jsonString) {
        assert Looper.getMainLooper().isCurrentThread() : "Called from non-UI thread";
        SimulatorBridge.postSettings(jsonString);
        SimulatorBridge.draw();
    }

    public String querySettings() {
        assert Looper.getMainLooper().isCurrentThread() : "Called from non-UI thread";
        return SimulatorBridge.querySettings();
    }
}
