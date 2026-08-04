#[cfg(feature = "video")]
use ac_ffmpeg::{
    codec::video::{
        frame::{PixelFormat, VideoFrame},
        scaler::{Algorithm, VideoFrameScaler},
        VideoDecoder,
    },
    format::{
        demuxer::{Demuxer, DemuxerWithStreamInfo},
        io::IO,
    },
    Error,
};
#[cfg(feature = "video")]
use std::convert::TryFrom;
#[cfg(feature = "video")]
use std::fs::File;
use std::path::Path;
use std::sync::{Arc, RwLock};

use vss::*;

#[cfg(feature = "video")]
fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a.max(1)
}

#[cfg(feature = "video")]
#[derive(Clone)]
pub struct VideoRenderState {
    pub generation: u64,
    pub eof: bool,
    pub pts: ac_ffmpeg::time::Timestamp,
    pub frame_time_base: ac_ffmpeg::time::TimeBase,
    pub decode_error: Option<String>,
}

pub struct UploadVideo {
    upload_start: Option<std::time::Instant>,
    uploader: UploadRgbBuffer,
    next_pts: f32,
    #[cfg(feature = "video")]
    next_timestamp: ac_ffmpeg::time::Timestamp,
    next_buffer: Arc<RgbBuffer>,
    input_size: Option<[u32; 2]>,
    shared_frame: Option<SharedVideoFrames>,
    #[cfg(feature = "video")]
    demuxer: Option<DemuxerWithStreamInfo<File>>,
    #[cfg(feature = "video")]
    video_stream_index: usize,
    #[cfg(feature = "video")]
    video_decoder: Option<VideoDecoder>,
    #[cfg(feature = "video")]
    video_scaler: Option<VideoFrameScaler>,
    #[cfg(feature = "video")]
    batch_state: Option<Arc<RwLock<VideoRenderState>>>,
    #[cfg(feature = "video")]
    advance_batch_frame: bool,
    #[cfg(feature = "video")]
    frame_time_base: ac_ffmpeg::time::TimeBase,
}

#[derive(Clone)]
pub struct SharedVideoFrames(Arc<RwLock<SharedVideoFrame>>);

struct SharedVideoFrame {
    generation: u64,
    buffer: Arc<RgbBuffer>,
    input_size: Option<[u32; 2]>,
}

impl UploadVideo {
    pub fn new(context: &RenderContext) -> Self {
        let uploader = UploadRgbBuffer::new(context);
        Self {
            upload_start: None,
            uploader,
            next_pts: -1.0,
            #[cfg(feature = "video")]
            next_timestamp: ac_ffmpeg::time::Timestamp::from_micros(0),
            next_buffer: Arc::new(RgbBuffer::default()),
            input_size: None,
            shared_frame: None,
            #[cfg(feature = "video")]
            demuxer: None,
            #[cfg(feature = "video")]
            video_stream_index: 0,
            #[cfg(feature = "video")]
            video_decoder: None,
            #[cfg(feature = "video")]
            video_scaler: None,
            #[cfg(feature = "video")]
            batch_state: None,
            #[cfg(feature = "video")]
            advance_batch_frame: false,
            #[cfg(feature = "video")]
            frame_time_base: ac_ffmpeg::time::TimeBase::new(1, 30),
        }
    }

    pub fn share_frames(&mut self) -> SharedVideoFrames {
        let shared = SharedVideoFrames(Arc::new(RwLock::new(SharedVideoFrame {
            generation: 0,
            buffer: self.next_buffer.clone(),
            input_size: self.input_size,
        })));
        self.shared_frame = Some(shared.clone());
        shared
    }

    pub fn has_video_extension<P>(path: P) -> bool
    where
        P: AsRef<Path>,
    {
        let extension = path.as_ref().extension().unwrap_or_default();
        extension == "avi" || extension == "mp4" || extension == "m4v" || extension == "mkv"
    }

    #[cfg(not(feature = "video"))]
    pub fn open<P>(&mut self, _path: P) -> Result<(), std::io::Error>
    where
        P: AsRef<Path> + std::fmt::Debug,
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Video feature is disabled",
        ))
    }

    #[cfg(feature = "video")]
    pub fn open<P>(&mut self, path: P) -> Result<(), Error>
    where
        P: AsRef<Path> + std::fmt::Debug,
    {
        // Open file.
        let file = File::open(&path)
            .map_err(|err| Error::new(format!("Failed to open input video {:?}: {}", path, err)))?;

        // Create demuxer for accessing streams.
        let io = IO::from_seekable_read_stream(file);
        let demuxer = Demuxer::builder()
            .build(io)?
            .find_stream_info(None)
            .map_err(|(_, err)| err)?;

        // Locate video stream and create a decoder.
        let (video_stream_index, video_params) = demuxer
            .streams()
            .iter()
            .map(|stream| stream.codec_parameters())
            .enumerate()
            .find(|(_, params)| params.is_video_codec())
            .ok_or_else(|| Error::new("Missing video stream"))?;
        let video_params = video_params.as_video_codec_parameters().unwrap();
        let stream = &demuxer.streams()[video_stream_index];
        if let Some(frame_count) = stream.frames() {
            let duration = stream.duration();
            if !duration.is_null() && duration.timestamp() > 0 {
                let numerator = duration.timestamp() * i64::from(duration.time_base().num());
                let denominator = i64::from(duration.time_base().den()) * frame_count as i64;
                let divisor = gcd(numerator.unsigned_abs(), denominator.unsigned_abs()) as i64;
                let numerator = numerator / divisor;
                let denominator = denominator / divisor;
                if let (Ok(numerator), Ok(denominator)) =
                    (i32::try_from(numerator), i32::try_from(denominator))
                {
                    self.frame_time_base = ac_ffmpeg::time::TimeBase::new(numerator, denominator);
                }
            }
        }
        if cfg!(debug_assertions) {
            println!(
                "Video codec: {}",
                video_params.decoder_name().unwrap_or("n/a")
            );
            println!(
                "Video format: {}x{} ({})",
                video_params.width(),
                video_params.height(),
                video_params.pixel_format().name()
            );
        }

        let video_decoder = VideoDecoder::from_codec_parameters(video_params)?.build()?;

        use std::str::FromStr;
        let target_format = PixelFormat::from_str("rgb0")
            .map_err(|err| Error::new(format!("Failed create target format {:?}", err)))?;
        let video_scaler = VideoFrameScaler::builder()
            .source_width(video_params.width())
            .source_height(video_params.height())
            .source_pixel_format(video_params.pixel_format())
            .target_width(video_params.width())
            .target_height(video_params.height())
            .target_pixel_format(target_format)
            .algorithm(Algorithm::FastBilinear)
            .build()?;
        let input_size = [
            u32::try_from(video_params.width())
                .map_err(|_| Error::new("Video width exceeds supported range"))?,
            u32::try_from(video_params.height())
                .map_err(|_| Error::new("Video height exceeds supported range"))?,
        ];
        self.uploader
            .set_render_resolution(RenderResolution::Custom { res: input_size });
        self.input_size = Some(input_size);

        self.upload_start = Some(std::time::Instant::now());
        self.demuxer = Some(demuxer);
        self.video_stream_index = video_stream_index;
        self.video_decoder = Some(video_decoder);
        self.video_scaler = Some(video_scaler);
        Ok(())
    }

    #[cfg(feature = "video")]
    pub fn enable_batch_render(&mut self) -> Result<Arc<RwLock<VideoRenderState>>, Error> {
        if !self.next_frame()? {
            return Err(Error::new("Video contains no decodable frames"));
        }
        let state = Arc::new(RwLock::new(VideoRenderState {
            generation: 1,
            eof: false,
            pts: ac_ffmpeg::time::Timestamp::from_micros(0),
            frame_time_base: self.frame_time_base,
            decode_error: None,
        }));
        state.write().unwrap().pts = self.next_timestamp;
        self.batch_state = Some(state.clone());
        Ok(state)
    }

    #[cfg(feature = "video")]
    fn next_frame(&mut self) -> Result<bool, Error> {
        use ac_ffmpeg::codec::Decoder;

        let mut result = Ok(false);
        if let Some(video_decoder) = &mut self.video_decoder {
            if let Some(video_scaler) = &mut self.video_scaler {
                loop {
                    // Try to decode one frame.
                    match video_decoder.take() {
                        Ok(Some(frame)) => {
                            // Process frame.
                            // XXX: using a software scaler might be a bad idea for 10bit 4k video data.
                            let scaled_frame = video_scaler.scale(&frame)?;
                            self.update_from_video_frame(scaled_frame);
                            result = Ok(true);
                            break;
                        }
                        Ok(None) => {
                            // Demux another packet.
                            let mut retry = false;
                            if let Some(demuxer) = &mut self.demuxer {
                                while let Some(packet) = demuxer.take()? {
                                    if packet.stream_index() == self.video_stream_index {
                                        video_decoder.push(packet)?;
                                        retry = true;
                                        break;
                                    }
                                }
                            }
                            if !retry {
                                break;
                            }
                        }
                        Err(err) => {
                            result = Err(err);
                            break;
                        }
                    }
                }
            }
        }
        result
    }

    #[cfg(feature = "video")]
    fn update_from_video_frame(&mut self, rgba_frame: VideoFrame) {
        let pts = rgba_frame.pts().as_f32().unwrap_or(0f32);
        self.next_timestamp = rgba_frame.pts();
        let width = rgba_frame.width() as u32;
        let height = rgba_frame.height() as u32;
        let plane0 = &rgba_frame.planes()[0];

        self.next_pts = pts;

        self.next_buffer = Arc::new(RgbBuffer {
            pixels_rgb: plane0.data().into(),
            width,
            height,
        });
        if let Some(shared) = &self.shared_frame {
            let mut frame = shared.0.write().unwrap();
            frame.generation += 1;
            frame.buffer = self.next_buffer.clone();
            frame.input_size = Some([width, height]);
        }
    }

    fn validate_data(&mut self) -> bool {
        let mut output_changed = false;

        #[cfg(feature = "video")]
        if let Some(state) = self.batch_state.clone() {
            if self.advance_batch_frame {
                self.advance_batch_frame = false;
                match self.next_frame() {
                    Ok(true) => {
                        let mut state = state.write().unwrap();
                        state.generation += 1;
                        state.pts = self.next_timestamp;
                        output_changed = true;
                    }
                    Ok(false) => state.write().unwrap().eof = true,
                    Err(err) => {
                        let mut state = state.write().unwrap();
                        state.eof = true;
                        state.decode_error = Some(err.to_string());
                    }
                }
            }
            if !state.read().unwrap().eof {
                self.uploader.upload_buffer(&self.next_buffer);
                output_changed = true;
            }
            return output_changed;
        }

        if let Some(upload_start) = self.upload_start {
            #[cfg(feature = "video")]
            if self.next_pts < 0.0 {
                output_changed |= self.next_frame().unwrap();
            }

            let current_pts = upload_start.elapsed().as_secs_f32();
            if self.next_pts >= 0.0 && self.next_pts <= current_pts {
                self.uploader.upload_buffer(&self.next_buffer);
                self.next_pts = -1.0;
                output_changed = true;
            }
        }
        output_changed
    }

    pub fn set_flags(&mut self, flags: RgbInputFlags) {
        self.uploader.set_flags(flags);
    }

    pub fn input_size(&self) -> Option<[u32; 2]> {
        self.input_size.or_else(|| self.uploader.input_size())
    }
}

pub struct UploadSharedVideo {
    uploader: UploadRgbBuffer,
    shared: SharedVideoFrames,
    uploaded_generation: u64,
}

impl UploadSharedVideo {
    pub fn new(context: &RenderContext, shared: SharedVideoFrames) -> Self {
        Self {
            uploader: UploadRgbBuffer::new(context),
            shared,
            uploaded_generation: 0,
        }
    }

    pub fn set_flags(&mut self, flags: RgbInputFlags) {
        self.uploader.set_flags(flags);
    }

    pub fn input_size(&self) -> Option<[u32; 2]> {
        self.shared.0.read().unwrap().input_size
    }

    fn synchronize(&mut self) -> bool {
        let frame = self.shared.0.read().unwrap();
        if frame.generation == self.uploaded_generation || frame.buffer.width == 0 {
            return false;
        }
        self.uploader.upload_buffer(&frame.buffer);
        self.uploaded_generation = frame.generation;
        true
    }
}

impl Node for UploadSharedVideo {
    fn name(&self) -> &'static str {
        "UploadSharedVideo"
    }

    fn negociate_slots(
        &mut self,
        context: &RenderContext,
        slots: NodeSlots,
        original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        self.synchronize();
        Node::negociate_slots(&mut self.uploader, context, slots, original_image)
    }

    fn input(&mut self, eye: &EyeInput, mouse: &MouseInput) -> (EyeInput, NodeChanges) {
        let changed = self.synchronize();
        let (eye, changes) = Node::input(&mut self.uploader, eye, mouse);
        (
            eye,
            (changes | NodeChanges::from_output_slots(changed, false)).normalized(),
        )
    }

    fn render(
        &mut self,
        context: &RenderContext,
        encoder: &mut wgpu::CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        self.synchronize();
        Node::render(&mut self.uploader, context, encoder, screen)
    }

    fn post_render(&mut self, context: &RenderContext) {
        Node::post_render(&mut self.uploader, context);
    }
}

impl Node for UploadVideo {
    fn name(&self) -> &'static str {
        "UploadVideo"
    }

    fn negociate_slots(
        &mut self,
        context: &RenderContext,
        slots: NodeSlots,
        original_image: &mut Option<Texture>,
    ) -> NodeSlots {
        self.validate_data();
        Node::negociate_slots(&mut self.uploader, context, slots, original_image)
    }

    fn input(&mut self, eye: &EyeInput, mouse: &MouseInput) -> (EyeInput, NodeChanges) {
        let output_changed = self.validate_data() || self.upload_start.is_some();
        #[cfg(feature = "video")]
        let output_changed = {
            let mut output_changed = output_changed;
            if let Some(state) = &self.batch_state {
                output_changed |= !state.read().unwrap().eof;
            }
            output_changed
        };
        let (eye, input_changes) = Node::input(&mut self.uploader, eye, mouse);
        (
            eye,
            (input_changes | NodeChanges::from_output_slots(output_changed, false)).normalized(),
        )
    }

    fn render(
        &mut self,
        context: &RenderContext,
        encoder: &mut wgpu::CommandEncoder,
        screen: Option<&RenderTexture>,
    ) {
        self.validate_data();
        Node::render(&mut self.uploader, context, encoder, screen)
    }

    fn post_render(&mut self, context: &RenderContext) {
        Node::post_render(&mut self.uploader, context);
        if self.upload_start.is_some() {
            context.apply_changes(NodeChanges::OUTPUT);
        }
        #[cfg(feature = "video")]
        if self.batch_state.is_some() {
            self.advance_batch_frame = true;
            context.apply_changes(NodeChanges::OUTPUT);
        }
    }
}
