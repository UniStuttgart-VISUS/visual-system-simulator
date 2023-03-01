package com.vss;

import android.content.Context;
import android.view.SurfaceHolder;
import android.view.SurfaceView;

import androidx.annotation.NonNull;

/**
 * Surface view for native library rendering.
 */
public class LibSurfaceView extends SurfaceView implements SurfaceHolder.Callback2 {

    private LibBridge libBridge;

    public LibSurfaceView(Context context) {
        super(context);

        this.setAlpha(1.0f);
        this.getHolder().addCallback(this);
    }

    @Override
    public void surfaceCreated(@NonNull SurfaceHolder holder) {
        LibBridge.create(holder.getSurface(), getResources().getAssets());
    }

    @Override
    public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
        LibBridge.resize(width, height);
    }

    @Override
    public void surfaceDestroyed(@NonNull SurfaceHolder holder) {
        LibBridge.destroy();
    }

    @Override
    public void surfaceRedrawNeededAsync(@NonNull SurfaceHolder holder, @NonNull Runnable drawingFinished) {
        LibBridge.draw();
        drawingFinished.run();
    }

    @Override
    public void surfaceRedrawNeeded(SurfaceHolder holder) {
        LibBridge.draw();
    }
}
