use super::VideoRenderState;
use ac_ffmpeg::{
    codec::{
        video::{
            frame::{get_pixel_format, VideoFrameMut},
            scaler::{Algorithm, VideoFrameScaler},
            VideoEncoder,
        },
        Encoder,
    },
    format::{
        io::IO,
        muxer::{Muxer, OutputFormat},
    },
    time::{TimeBase, Timestamp},
    Error,
};
use std::{
    fs::{File, OpenOptions},
    mem::size_of,
    path::PathBuf,
    sync::{Arc, RwLock},
};
use vss::*;

struct OutputPipeline {
    encoder: VideoEncoder,
    scaler: VideoFrameScaler,
    muxer: Muxer<File>,
    time_base: TimeBase,
}

impl OutputPipeline {
    fn open(
        path: &PathBuf,
        width: usize,
        height: usize,
        time_base: TimeBase,
        force: bool,
    ) -> Result<Self, Error> {
        let rgb24 = get_pixel_format("rgb24");
        let yuv420p = get_pixel_format("yuv420p");
        let encoder = VideoEncoder::builder("libx264")?
            .pixel_format(yuv420p)
            .width(width)
            .height(height)
            .time_base(time_base)
            .build()?;
        let codec_parameters = encoder.codec_parameters().into();
        let output_format = OutputFormat::guess_from_file_name(path.to_string_lossy().as_ref())
            .ok_or_else(|| {
                Error::new(format!(
                    "Unable to determine video format for {}",
                    path.display()
                ))
            })?;
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|err| {
                    Error::new(format!(
                        "Unable to create output directory {}: {}",
                        parent.display(),
                        err
                    ))
                })?;
            }
        }
        let output = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(force)
            .create_new(!force)
            .open(path)
            .map_err(|err| Error::new(format!("Unable to create {}: {}", path.display(), err)))?;
        let mut muxer_builder = Muxer::builder();
        muxer_builder.add_stream(&codec_parameters)?;
        let muxer = muxer_builder.build(IO::from_seekable_write_stream(output), output_format)?;
        let scaler = VideoFrameScaler::builder()
            .source_pixel_format(rgb24)
            .source_width(width)
            .source_height(height)
            .target_pixel_format(yuv420p)
            .target_width(width)
            .target_height(height)
            .algorithm(Algorithm::Bilinear)
            .build()?;
        Ok(Self {
            encoder,
            scaler,
            muxer,
            time_base,
        })
    }

    fn push(&mut self, rgb: RgbBuffer, pts: Timestamp) -> Result<(), Error> {
        let width = rgb.width as usize;
        let mut frame = VideoFrameMut::black(get_pixel_format("rgb24"), width, rgb.height as usize)
            .with_time_base(self.time_base)
            .with_pts(pts.with_time_base(self.time_base));
        let mut planes = frame.planes_mut();
        for (target, source) in planes[0]
            .lines_mut()
            .zip(rgb.pixels_rgb.chunks_exact(width * 3))
        {
            target[..source.len()].copy_from_slice(source);
        }
        let frame = self.scaler.scale(&frame.freeze())?;
        self.encoder.push(frame)?;
        while let Some(packet) = self.encoder.take()? {
            self.muxer
                .push(packet.with_raw_duration(1).with_stream_index(0))?;
        }
        Ok(())
    }

    fn close(mut self) -> Result<(), Error> {
        self.encoder.flush()?;
        while let Some(packet) = self.encoder.take()? {
            self.muxer
                .push(packet.with_raw_duration(1).with_stream_index(0))?;
        }
        self.muxer.flush()
    }
}

pub struct DownloadVideo {
    input: Texture,
    buffer: wgpu::Buffer,
    res: [f32; 2],
    output_path: PathBuf,
    force: bool,
    pipeline: Option<OutputPipeline>,
    state: Arc<RwLock<VideoRenderState>>,
    last_generation: u64,
    processed: Arc<RwLock<bool>>,
    failure: Arc<RwLock<Option<String>>>,
}

impl DownloadVideo {
    pub fn new(
        context: &RenderContext,
        output_path: PathBuf,
        force: bool,
        state: Arc<RwLock<VideoRenderState>>,
        processed: Arc<RwLock<bool>>,
        failure: Arc<RwLock<Option<String>>>,
    ) -> Self {
        let dimensions = BufferDimensions::new(1, 1, size_of::<u32>());
        let buffer = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Download Video Placeholder Buffer"),
            size: (dimensions.padded_bytes_per_row * dimensions.height) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let input = placeholder_texture(
            context.device(),
            context.queue(),
            Some("Download video placeholder"),
        )
        .unwrap();
        Self {
            input,
            buffer,
            res: [0.0; 2],
            output_path,
            force,
            pipeline: None,
            state,
            last_generation: 0,
            processed,
            failure,
        }
    }

    fn fail(&self, message: impl ToString) {
        *self.failure.write().unwrap() = Some(message.to_string());
    }
}

impl Node for DownloadVideo {
    fn name(&self) -> &'static str {
        "DownloadVideo"
    }

    fn negociate_slots(
        &mut self,
        context: &RenderContext,
        slots: NodeSlots,
        _original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        let slots = slots.to_color_input(context);
        self.res = slots.input_size_f32();
        (self.input, _) = slots.as_color_source(context.device());
        let dimensions =
            BufferDimensions::new(self.res[0] as usize, self.res[1] as usize, size_of::<u32>());
        self.buffer = context.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Download Video Buffer"),
            size: (dimensions.padded_bytes_per_row * dimensions.height) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let time_base = self.state.read().unwrap().frame_time_base;
        match OutputPipeline::open(
            &self.output_path,
            dimensions.width,
            dimensions.height,
            time_base,
            self.force,
        ) {
            Ok(pipeline) => self.pipeline = Some(pipeline),
            Err(err) => self.fail(err),
        }
        slots
    }

    fn render(
        &mut self,
        _context: &RenderContext,
        encoder: &mut wgpu::CommandEncoder,
        _screen: Option<&RenderTexture>,
    ) {
        let state = self.state.read().unwrap();
        if state.generation == self.last_generation {
            return;
        }
        let dimensions =
            BufferDimensions::new(self.res[0] as usize, self.res[1] as usize, size_of::<u32>());
        encoder.copy_texture_to_buffer(
            self.input.texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &self.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(dimensions.padded_bytes_per_row as u32),
                    rows_per_image: None,
                },
            },
            wgpu::Extent3d {
                width: dimensions.width as u32,
                height: dimensions.height as u32,
                depth_or_array_layers: 1,
            },
        );
    }

    fn post_render(&mut self, context: &RenderContext) {
        if self.failure.read().unwrap().is_some() {
            return;
        }
        let state = self.state.read().unwrap().clone();
        if let Some(message) = &state.decode_error {
            self.fail(format!("Failed to decode video frame: {message}"));
            return;
        }
        if state.generation != self.last_generation {
            let slice = self.buffer.slice(..);
            slice.map_async(wgpu::MapMode::Read, |_| {});
            if let Err(err) = context.device().poll(wgpu::PollType::wait_indefinitely()) {
                self.fail(format!("Failed waiting for video frame download: {err}"));
                return;
            }
            let dimensions =
                BufferDimensions::new(self.res[0] as usize, self.res[1] as usize, size_of::<u32>());
            let padded = slice.get_mapped_range();
            let mut pixels = Vec::with_capacity(dimensions.width * dimensions.height * 3);
            for row in padded.chunks(dimensions.padded_bytes_per_row) {
                for i in (0..dimensions.unpadded_bytes_per_row).step_by(4) {
                    pixels.extend_from_slice(&row[i..i + 3]);
                }
            }
            drop(padded);
            self.buffer.unmap();
            let rgb = RgbBuffer {
                pixels_rgb: pixels.into_boxed_slice(),
                width: dimensions.width as u32,
                height: dimensions.height as u32,
            };
            if let Some(pipeline) = &mut self.pipeline {
                let pts = Timestamp::new((state.generation - 1) as i64, state.frame_time_base);
                if let Err(err) = pipeline.push(rgb, pts) {
                    self.fail(err);
                    return;
                }
            }
            self.last_generation = state.generation;
        }
        if state.eof {
            if let Some(pipeline) = self.pipeline.take() {
                if let Err(err) = pipeline.close() {
                    self.fail(err);
                    return;
                }
            }
            *self.processed.write().unwrap() = true;
        }
    }
}
