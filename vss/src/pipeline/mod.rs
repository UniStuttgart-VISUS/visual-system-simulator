mod pass;
mod texture;
#[macro_use]
mod utils;

pub use self::pass::*;
pub use self::texture::*;
pub use self::utils::*;

use gfx::traits::FactoryExt;
use gfx::Factory;

use crate::devices::*;
use crate::passes::*;

/// The pipeline encapsulates the simulation and rendering system, i.e., all rendering passes.
pub struct Pipeline {
    passes: Vec<Box<dyn Pass>>,
    targets: Vec<gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>>,
    params: ValueMap,
}

impl Pipeline {
    pub fn new(device: &mut Box<dyn Device>, params: ValueMap) -> Self {
        Pipeline {
            passes: Vec::new(),
            targets: Vec::new(),
            params,
        }
        .create_passes(device)
        .create_intermediate_buffers(device)
    }

    fn create_passes(mut self, device: &mut Box<dyn Device>) -> Self {
        let factory = &mut device.factory().borrow_mut() as &mut gfx_device_gl::Factory;

        //XXX: some more unification would be nice.

        // Color conversion passes.
        let source: &DeviceSource = &device.source().borrow();
        if let DeviceSource::Yuv { .. } = source {
            #[cfg(target_os = "android")]
            {
                self.renderers.push(Box::new(Yuv420Rgb::new(factory)));
            }
            #[cfg(not(target_os = "android"))]
            {
                self.passes.push(Box::new(YuvRgb::new(factory)));
            }
        }

        // Visual system passes.
        self.passes.push(Box::new(Cataract::new(factory)));
        #[cfg(not(target_os = "android"))]
        {
            self.passes.push(Box::new(Lens::new(factory)));
        }
        self.passes.push(Box::new(Retina::new(factory)));

        self
    }

    fn create_intermediate_buffers(mut self, device: &mut Box<dyn Device>) -> Self {
        let mut factory = device.factory().borrow_mut();

        let device_source = device.source().borrow_mut();
        let device_target = device.target().borrow_mut();
        let (width, height, _, _) = device_target.get_dimensions();

        struct ViewPair<T: gfx::format::Formatted> {
            pub source: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, T::View>,
            pub target: gfx::handle::RenderTargetView<gfx_device_gl::Resources, T>,
        }

        let intermediate1 = {
            let (_, srv, rtv) = factory.create_render_target(width, height).unwrap();
            ViewPair {
                source: srv,
                target: rtv,
            }
        };
        let intermediate2 = {
            let (_, srv, rtv) = factory.create_render_target(width, height).unwrap();
            ViewPair {
                source: srv,
                target: rtv,
            }
        };

        let source_sampler = factory.create_sampler_linear();
        let amount = self.passes.len();
        for (idx, pass) in &mut self.passes.iter_mut().enumerate() {
            let count = idx + 1;

            // Update its source and target.
            let source = if count == 1 {
                device_source.clone()
            } else {
                let source = if count % 2 == 0 {
                    &intermediate1.source
                } else {
                    &intermediate2.source
                };
                DeviceSource::Rgb {
                    rgba8: source.clone(),
                }
            };
            let target = if count == amount {
                &(device_target)
            } else if count % 2 == 0 {
                &intermediate2.target
            } else {
                &intermediate1.target
            };
            let stereo = (count == amount)
                && self
                    .params
                    .get("split_screen_switch")
                    .unwrap_or(&Value::Bool(false))
                    .as_bool()
                    .unwrap_or(false);
            pass.update_io(
                target,
                (width as u32, height as u32),
                &source,
                &source_sampler,
                (width as u32, height as u32),
                stereo,
            );
            self.targets.push(target.clone());

            // Update its values.
            pass.update_params(&mut factory, &self.params);
        }

        self
    }

    pub fn update_params(&mut self, device: &mut Box<dyn Device>) {
        let mut factory = device.factory().borrow_mut();

        // Propagate to passes.
        for pass in &mut self.passes {
            pass.update_params(&mut factory, &self.params);
        }
    }

    pub fn render(&mut self, device: &mut Box<dyn Device>) {
        let mut factory = device.factory().borrow_mut();

        //XXX: do this only on demand
        let device_target = device.target().borrow_mut();
        let (width, height, _, _) = device_target.get_dimensions();
        let source = &device.source().borrow();
        let source_sampler = factory.create_sampler_linear();
        self.passes[0].update_io(
            &self.targets[0],
            (width as u32, height as u32),
            &source,
            &source_sampler,
            (width as u32, height as u32),
            false,
        );

        // Render all passes.
        let mut encoder = device.encoder().borrow_mut();
        for pass in &mut self.passes {
            pass.render(&mut encoder, &device.gaze());
        }
    }
}
