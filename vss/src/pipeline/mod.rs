mod pass;
mod texture;
#[macro_use]
mod utils;

pub use self::pass::*;
pub use self::texture::*;
pub use self::utils::*;

use gfx::traits::FactoryExt;
use gfx::Factory;
use std::cell::RefCell;

use crate::devices::*;

/// The pipeline encapsulates the simulation and rendering system, i.e., all rendering passes.
pub struct Pipeline {
    passes: Vec<RefCell<Box<dyn Pass>>>,
    targets: RefCell<Vec<gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>>>,
}

impl Pipeline {
    pub fn new() -> Self {
        Pipeline {
            passes: Vec::new(),
            targets: RefCell::new(Vec::new()),
        }
    }

    pub fn add<P>(&mut self, device: &dyn Device)
    where
        P: Pass + 'static,
    {
        let mut factory = device.factory().borrow_mut();
        self.passes
            .push(RefCell::new(Box::new(P::build(&mut factory))));
    }

    pub fn update_io(&self, device: &dyn Device) {
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
        for (idx, pass) in self.passes.iter().enumerate() {
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

            pass.borrow_mut().update_io(
                target,
                (width as u32, height as u32),
                &source,
                &source_sampler,
                (width as u32, height as u32),
            );
            self.targets.borrow_mut().push(target.clone());
        }
    }

    pub fn update_params(&self, device: &dyn Device, params: &ValueMap) {
        let mut factory = device.factory().borrow_mut();

        // Propagate to passes.
        for pass in self.passes.iter() {
            pass.borrow_mut().update_params(&mut factory, &params);
        }
    }

    pub fn render(&self, device: &dyn Device) {
        let mut factory = device.factory().borrow_mut();

        //XXX: do this only on demand
        let device_target = device.target().borrow_mut();
        let (width, height, _, _) = device_target.get_dimensions();
        let source = &device.source().borrow();
        let source_sampler = factory.create_sampler_linear();
        self.passes[0].borrow_mut().update_io(
            &self.targets.borrow()[0],
            (width as u32, height as u32),
            &source,
            &source_sampler,
            (width as u32, height as u32),
        );

        // Render all passes.
        let mut encoder = device.encoder().borrow_mut();
        for pass in self.passes.iter() {
            pass.borrow_mut().render(&mut encoder, &device.gaze());
        }
    }
}
