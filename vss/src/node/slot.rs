use super::*;
use gfx;

pub type ColorFormat = (gfx::format::R8_G8_B8_A8, gfx::format::Unorm);
pub type DepthFormat = (gfx::format::R32, gfx::format::Float);

#[derive(Clone)]
pub enum Slot {
    Empty,
    Rgb {
        color: gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>,
        color_view: Option<gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>>, //TODO: drop last component?
    },
    RgbDepth {
        color: gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>,
        color_view: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>, //TODO: drop last component?
        depth: gfx::handle::RenderTargetView<gfx_device_gl::Resources, DepthFormat>,
        depth_view: gfx::handle::ShaderResourceView<gfx_device_gl::Resources, f32>,
    },
    // XXX: Stereo
}
impl Default for Slot {
    fn default() -> Self {
        Slot::Empty
    }
}

#[derive(Clone)]
pub struct NodeSlots {
    input: Slot,
    output: Slot,
    sampler: gfx::handle::Sampler<gfx_device_gl::Resources>,
}

impl NodeSlots {
    pub fn new(window: &Window) -> Self {
        Self {
            input: Slot::default(),
            output: Slot::default(),
            sampler: window.factory().borrow_mut().create_sampler_linear(),
        }
    }

    pub fn new_io(window: &Window, input: Slot, output: Slot) -> Self {
        Self {
            input,
            output,
            sampler: window.factory().borrow_mut().create_sampler_linear(),
        }
    }

    pub fn take_input(&mut self) -> Slot {
        std::mem::take(&mut self.input)
    }

    pub fn take_output(&mut self) -> Slot {
        std::mem::take(&mut self.output)
    }

    pub fn to_passthrough(self) -> Self {
        Self {
            input: Slot::Empty,
            output: self.input,
            sampler: self.sampler,
        }
    }

    pub fn to_color_input(self, _window: &Window) -> Self {
        match self.input {
            Slot::Empty => {
                panic!("Input expected");
            }
            Slot::Rgb { .. } => self,
            Slot::RgbDepth {
                color, color_view, ..
            } => Self {
                input: Slot::Rgb {
                    color,
                    color_view: Some(color_view),
                },
                output: self.output,
                sampler: self.sampler,
            },
        }
    }

    pub fn to_color_depth_input(self, _window: &Window) -> Self {
        match self.input {
            Slot::Empty => {
                panic!("Input expected");
            }
            Slot::Rgb { .. } => {
                panic!("RGB input cannot be extended with depth");
            }
            Slot::RgbDepth { .. } => self,
        }
    }

    pub fn to_color_output(self, window: &Window) -> Self {
        match self.output {
            Slot::Empty => {
                // Guess output size, based on input.
                let (width, height, ..) = match &self.input {
                    Slot::Empty => {
                        panic!("Input expected");
                    }
                    Slot::Rgb { color, .. } => color.get_dimensions(),
                    Slot::RgbDepth { color, .. } => color.get_dimensions(),
                };
                let mut factory = window.factory().borrow_mut();
                let (color, color_view) = create_texture_render_target::<ColorFormat>(
                    &mut factory,
                    width as u32,
                    height as u32,
                );

                Self {
                    input: self.input,
                    output: Slot::Rgb {
                        color,
                        color_view: Some(color_view),
                    },
                    sampler: self.sampler,
                }
            }
            Slot::Rgb { .. } => self,
            Slot::RgbDepth {
                color, color_view, ..
            } => Self {
                input: self.input,
                output: Slot::Rgb {
                    color,
                    color_view: Some(color_view),
                },
                sampler: self.sampler,
            },
        }
    }

    pub fn to_color_depth_output(self, window: &Window) -> Self {
        match self.output {
            Slot::Empty => {
                // Guess output, based on input.
                let (width, height, ..) = match &self.input {
                    Slot::Empty => {
                        panic!("Input expected");
                    }
                    Slot::Rgb { color, .. } => color.get_dimensions(),
                    Slot::RgbDepth { color, .. } => color.get_dimensions(),
                };
                let mut factory = window.factory().borrow_mut();
                let (color, color_view) = create_texture_render_target::<ColorFormat>(
                    &mut factory,
                    width as u32,
                    height as u32,
                );
                let (depth, depth_view) = create_texture_render_target::<DepthFormat>(
                    &mut factory,
                    width as u32,
                    height as u32,
                );

                Self {
                    input: self.input,
                    output: Slot::RgbDepth {
                        color,
                        color_view: color_view,
                        depth,
                        depth_view,
                    },
                    sampler: self.sampler,
                }
            }
            Slot::Rgb { color, color_view } => {
                // Guess missing depth, based on color.
                let mut factory = window.factory().borrow_mut();
                let (width, height, ..) = color.get_dimensions();
                let (depth, depth_view) = create_texture_render_target::<DepthFormat>(
                    &mut factory,
                    width as u32,
                    height as u32,
                );
                Self {
                    input: self.input,
                    output: Slot::RgbDepth {
                        color,
                        color_view: color_view.expect("Shader resource expected"),
                        depth,
                        depth_view,
                    },
                    sampler: self.sampler,
                }
            }
            Slot::RgbDepth { .. } => self,
        }
    }

    pub fn emplace_color_output(self, window: &Window, width: u32, height: u32) -> Self {
        let (color, color_view) = create_texture_render_target::<ColorFormat>(
            &mut window.factory().borrow_mut(),
            width,
            height,
        );
        Self {
            input: self.input,
            output: Slot::Rgb {
                color,
                color_view: Some(color_view),
            },
            sampler: self.sampler,
        }
    }

    pub fn emplace_color_depth_output(self, window: &Window, width: u32, height: u32) -> Self {
        let (color, color_view) = create_texture_render_target::<ColorFormat>(
            &mut window.factory().borrow_mut(),
            width,
            height,
        );
        let (depth, depth_view) = create_texture_render_target::<DepthFormat>(
            &mut window.factory().borrow_mut(),
            width,
            height,
        );

        Self {
            input: self.input,
            output: Slot::RgbDepth {
                color,
                color_view,
                depth,
                depth_view,
            },
            sampler: self.sampler,
        }
    }

    pub fn as_color(&self) -> gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat> {
        match &self.output {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB output expected");
            }
            Slot::Rgb { color, .. } => color.clone(),
        }
    }

    pub fn as_color_view(
        &self,
    ) -> (
        gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
        gfx::handle::Sampler<gfx_device_gl::Resources>,
    ) {
        match &self.input {
            Slot::Empty | Slot::RgbDepth { .. } => {
                panic!("RGB input expected");
            }
            Slot::Rgb { color_view, .. } => (
                color_view.clone().expect("Shader resource expected"),
                self.sampler.clone(),
            ),
        }
    }

    pub fn as_color_depth(
        &self,
    ) -> (
        gfx::handle::RenderTargetView<gfx_device_gl::Resources, ColorFormat>,
        gfx::handle::RenderTargetView<gfx_device_gl::Resources, DepthFormat>,
    ) {
        match &self.output {
            Slot::Empty | Slot::Rgb { .. } => {
                panic!("RGBD output expected");
            }
            Slot::RgbDepth { color, depth, .. } => (color.clone(), depth.clone()),
        }
    }

    pub fn as_color_depth_view(
        &self,
    ) -> (
        (
            gfx::handle::ShaderResourceView<gfx_device_gl::Resources, [f32; 4]>,
            gfx::handle::Sampler<gfx_device_gl::Resources>,
        ),
        (
            gfx::handle::ShaderResourceView<gfx_device_gl::Resources, f32>,
            gfx::handle::Sampler<gfx_device_gl::Resources>,
        ),
    ) {
        match &self.input {
            Slot::Empty | Slot::Rgb { .. } => {
                panic!("RGBD input expected");
            }
            Slot::RgbDepth {
                color_view,
                depth_view,
                ..
            } => (
                (color_view.clone(), self.sampler.clone()),
                (depth_view.clone(), self.sampler.clone()),
            ),
        }
    }

    fn output_size(&self) -> [u32; 2] {
        let dimensions = match &self.output {
            Slot::Empty => {
                panic!("Output expected");
            }
            Slot::Rgb { color, .. } => color.get_dimensions(),
            Slot::RgbDepth { color, .. } => color.get_dimensions(),
        };

        [dimensions.0 as u32, dimensions.1 as u32]
    }

    pub fn output_size_f32(&self) -> [f32; 2] {
        let size = self.output_size();
        [size[0] as f32, size[1] as f32]
    }

    pub fn input_size_f32(&self) -> [f32; 2] {
        let dimensions = match &self.input {
            Slot::Empty => {
                panic!("Output expected");
            }
            Slot::Rgb { color, .. } => color.get_dimensions(),
            Slot::RgbDepth { color, .. } => color.get_dimensions(),
        };

        [dimensions.0 as f32, dimensions.1 as f32]
    }
}
