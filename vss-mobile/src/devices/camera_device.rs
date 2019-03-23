
use device::{Device, OpenGlBundle};
use tools::core;
use gfx;
use gfx::Device;
use gfx_device_gl::{Resources, Factory};
use gfx::Factory as PFactory;
use glutin::GlContext;
use config::CoreConfig;

use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

use std::{thread, time};

use image::math::utils::clamp;
use core::SimulationCore;
use core::View;
use core::Texture;

use android::camera_ffi::channel;
use android::camera_ffi::Frame;
use std::ptr;

use std::time::{Duration, Instant};

use std::io::Cursor;
use std::io::prelude::*;

use gfx::format::Formatted;
use gfx::format::SurfaceTyped;
use gfx_device_gl;
use gfx::memory::Typed;
use gfx::traits::FactoryExt;

use gfx_gl as gl;
use std;

use std::slice;

use tools::remote_control;
use tools::remote_control::PreviewFrame;

pub struct CameraDevice{
    config: CoreConfig,
}


impl CameraDevice{
    pub fn new(config: &CoreConfig) -> CameraDevice where Self: Sized {
        CameraDevice {
            config: config.clone(),
        }
    }
}

impl Device for CameraDevice{

    fn get_config(&self) -> &CoreConfig {
        &self.config
    }

    fn init(&self, bundle: &mut Factory) {

    }

    fn get_input(&self, factory: &mut Factory)
        -> Result<(Texture,View), String> {
        let memory_y = vec![128;1920*1080].into_boxed_slice();
        let memory_u = vec![128;1920*1080].into_boxed_slice();
        let memory_v = vec![128;1920*1080].into_boxed_slice();

        let (t_y, v_y) = core::load_single_channel_texture_from_bytes(factory, Box::from(memory_y), 1920, 1080).unwrap();
        let (t_u, v_u) = core::load_single_channel_texture_from_bytes(factory, Box::from(memory_u), 1920, 1080).unwrap();
        let (t_v, v_v) = core::load_single_channel_texture_from_bytes(factory, Box::from(memory_v), 1920, 1080).unwrap();

        let view = DeviceSource::Yuv {
            y: v_y,
            u: v_u,
            v: v_v
        };

        let texture = Texture::Yuv {
            y: t_y,
            u: t_u,
            v: t_v
        };
        Ok((texture,view))
    }

    fn enter_loop(&self, mut bundle: OpenGlBundle,  mut core: SimulationCore) {
        let mut running = true;

        let mut width = 1920;
        let mut array_width = 1920;
        let mut height = 1080;
        //let mut memory = vec![0;width*height].into_boxed_slice();
        let mut memory_y = vec![0;width*height].into_boxed_slice();
        let mut memory_u = vec![0;width*height].into_boxed_slice();
        let mut memory_v = vec![0;width*height].into_boxed_slice();

        let mut lastFpsTime = Instant::now();
        let mut frameCount = 0;

        let mut accLockTime = Duration::new(0,0);
        let mut accConvTime = Duration::new(0,0);
        let mut accRenderTime = Duration::new(0,0);
        let mut accFlushTime = Duration::new(0,0);


        let size = width as usize *height as usize *4 as usize;

        {
            let mut preview = remote_control::FRAME.write().unwrap_or_else(|e|{panic!("locking problem, should not happen")});
            preview.buffer = vec![0; size];
        }

        //let download = bundle.factory.create_download_buffer(width as usize * height as usize).unwrap();

        let mut target = vec![42 as u8;1920*1080*4];


        while running {


            let mut curLockTime = Instant::now();

            self.handle_events(&mut bundle,&mut core,&mut running);

            let rx = &channel.1;

            let mut result = rx.lock().unwrap().try_recv();

            if result.is_err() {
                //println!("RCV ERR: {}",result.err().unwrap());
                //println!(" -- bad-data, skip");
                continue;
            }

            accLockTime += curLockTime.elapsed();


            let frame=result.unwrap();

            //-----------
            //let arr = vec![frame.y,frame.u,frame.v];

            //needed for correction in case the array is longer then it should be
            if (frame.y.len() != array_width*height) {
                array_width = frame.y.len()/height;
                println!("### resize");
            }

            /*
            We get 3 arays, y, u, v, with y twice the size as the others.
            The y values are per pixel the u an v values are for two pixels each.
            On android this is tricky, see below
            */

            let mut curConvTime = Instant::now();



            (&mut *memory_y).write(&*frame.y);
            (&mut *memory_u).write(&*frame.u);
            (&mut *memory_v).write(&*frame.v);

            accConvTime += curConvTime.elapsed();


            let size = [1920 as u16, 1080 as u16];

            core.render_yuv(&mut bundle.encoder, &mut bundle.factory, &memory_y, &memory_u, &memory_v, Some(size));
            //core.render(&mut bundle.encoder, &mut bundle.factory, &memory_y, Some(size));


            bundle.encoder.flush(&mut bundle.device);


            bundle.window.swap_buffers().unwrap();
            bundle.device.cleanup();





            frameCount = ( frameCount + 1 ) % 30;

        }

        println!("Exit of inf loop")
    }
}
