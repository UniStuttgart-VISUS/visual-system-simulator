package com.vss.simulator;

import android.content.Context;
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

    public void postFrame(int width, int height, byte[] y, byte[] u, byte[] v) {
        assert Looper.getMainLooper().isCurrentThread() : "Called from non-UI thread";
        SimulatorBridge.postFrame(width, height, y, u, v);
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
