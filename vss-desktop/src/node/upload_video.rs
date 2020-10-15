#[cfg(feature = "video")]
use ac_ffmpeg::{
    codec::video::{
        frame::{PixelFormat, VideoFrame},
        scaler::{Algorithm, VideoFrameScaler},
        VideoDecoder,
    },
    format::{
        demuxer::{Demuxer, DemuxerWithCodecParameters},
        io::IO,
    },
    Error,
};
#[cfg(feature = "video")]
use std::fs::File;
use std::path::Path;

use vss::*;

pub struct UploadVideo {
    upload_start: Option<std::time::Instant>,
    uploader: UploadRgbBuffer,
    next_pts: f32,
    next_buffer: RgbBuffer,
    #[cfg(feature = "video")]
    demuxer: Option<DemuxerWithCodecParameters<File>>,
    #[cfg(feature = "video")]
    video_stream_index: usize,
    #[cfg(feature = "video")]
    video_decoder: Option<VideoDecoder>,
    #[cfg(feature = "video")]
    video_scaler: Option<VideoFrameScaler>,
}

impl UploadVideo {
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
            .codec_parameters()
            .iter()
            .enumerate()
            .find(|(_, params)| params.is_video_codec())
            .ok_or_else(|| Error::new("Missing video stream"))?;
        let video_params = video_params.as_video_codec_parameters().unwrap();
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

        self.upload_start = Some(std::time::Instant::now());
        self.demuxer = Some(demuxer);
        self.video_stream_index = video_stream_index;
        self.video_decoder = Some(video_decoder);
        self.video_scaler = Some(video_scaler);
        Ok(())
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
                            self.from_video_frame(scaled_frame);
                            result = Ok(true);
                            break;
                        }
                        Ok(None) => {
                            // Demux another packet.
                            if let Some(demuxer) = &mut self.demuxer {
                                while let Some(packet) = demuxer.take()? {
                                    if packet.stream_index() == self.video_stream_index {
                                        video_decoder.push(packet)?;
                                        break;
                                    }
                                }
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
    fn from_video_frame(&mut self, rgba_frame: VideoFrame) {
        let pts = rgba_frame.pts().as_f32().unwrap_or(0f32);
        let width = rgba_frame.width() as u32;
        let height = rgba_frame.height() as u32;
        let plane0 = &rgba_frame.planes()[0];

        self.next_pts = pts;

        // Test if we have to invalidate the buffer.
        if self.next_buffer.width != width || self.next_buffer.height != height {
            // Reallocate and copy.
            self.next_buffer = RgbBuffer {
                pixels_rgb: plane0.data().into(),
                width,
                height,
            }
        } else {
            // Copy.
            self.next_buffer.pixels_rgb.copy_from_slice(&plane0.data());
        }
    }

    fn validate_data(&mut self) {
        if let Some(upload_start) = self.upload_start {
            #[cfg(feature = "video")]
            if self.next_pts < 0.0 {
                self.next_frame().unwrap();
            }

            let current_pts = upload_start.elapsed().as_secs_f32();
            if self.next_pts >= 0.0 && self.next_pts <= current_pts {
                self.uploader.upload_buffer(&self.next_buffer);
                self.next_pts = -1.0;
            }
        }
    }

    pub fn set_flags(&mut self, flags: RgbInputFlags) {
        self.uploader
            .set_flags(flags | RgbInputFlags::VERTICALLY_FLIPPED);
    }
}

impl Node for UploadVideo {
    fn new(window: &Window) -> Self {
        let mut uploader = UploadRgbBuffer::new(window);
        uploader.set_flags(RgbInputFlags::VERTICALLY_FLIPPED);
        Self {
            upload_start: None,
            uploader,
            next_pts: -1.0,
            next_buffer: RgbBuffer::default(),
            #[cfg(feature = "video")]
            demuxer: None,
            #[cfg(feature = "video")]
            video_stream_index: 0,
            #[cfg(feature = "video")]
            video_decoder: None,
            #[cfg(feature = "video")]
            video_scaler: None,
        }
    }

    fn update_io(
        &mut self,
        window: &Window,
        source: (Option<NodeSource>, Option<NodeTarget>),
        target_candidate: (Option<NodeSource>, Option<NodeTarget>),
    ) -> (Option<NodeSource>, Option<NodeTarget>) {
        self.validate_data();
        self.uploader.update_io(window, source, target_candidate)
    }

    fn update_values(&mut self, window: &Window, values: &ValueMap) {
        self.uploader.update_values(window, values);
    }

    fn input(&mut self, head: &Head, gaze: &Gaze) -> Gaze {
        self.uploader.input(head, gaze)
    }

    fn render(&mut self, window: &Window) {
        self.validate_data();
        self.uploader.render(window)
    }
}
