package com.vss.simulator;

import android.content.Context;
import android.util.AttributeSet;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.webkit.JavascriptInterface;

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
        SimulatorBridge.create(holder.getSurface(), getResources().getAssets());
    }

    @Override
    public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
        SimulatorBridge.resize(width, height);
    }

    @Override
    public void surfaceDestroyed(@NonNull SurfaceHolder holder) {
        SimulatorBridge.destroy();
    }

    @Override
    public void surfaceRedrawNeededAsync(@NonNull SurfaceHolder holder, @NonNull Runnable drawingFinished) {
        SimulatorBridge.draw();
        drawingFinished.run();
    }

    @Override
    public void surfaceRedrawNeeded(SurfaceHolder holder) {
        SimulatorBridge.draw();
    }

    @JavascriptInterface
    public void postSettings(String jsonString) {
        SimulatorBridge.postSettings(jsonString);
    }

    public void postFrame(int width, int height, byte[] y, byte[] u, byte[] v) {
        SimulatorBridge.postFrame(width, height, y, u, v);
    }
}
