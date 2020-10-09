#[cfg(feature = "video")]
use ac_ffmpeg::{
    codec::video::{frame::VideoFrame, VideoDecoder},
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
    uploader: UploadYuvBuffer,
    #[cfg(feature = "video")]
    demuxer: Option<DemuxerWithCodecParameters<File>>,
    #[cfg(feature = "video")]
    video_stream_index: usize,
    #[cfg(feature = "video")]
    video_decoder: Option<VideoDecoder>,
}

impl UploadVideo {
    pub fn has_video_extension<P>(path: P) -> bool
    where
        P: AsRef<Path>,
    {
        let extension = path.as_ref().extension().unwrap_or_default();
        if extension == "avi" {
            true
        } else if extension == "mp4" {
            true
        } else {
            false
        }
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
        let video_decoder = VideoDecoder::from_codec_parameters(video_params)?.build()?;

        self.demuxer = Some(demuxer);
        self.video_stream_index = video_stream_index;
        self.video_decoder = Some(video_decoder);
        Ok(())
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
    fn next_frame(&mut self) -> Result<Option<YuvBuffer>, Error> {
        use ac_ffmpeg::codec::Decoder;

        let mut result = Ok(None);
        if let Some(video_decoder) = &mut self.video_decoder {
            let mut retried = false;
            loop {
                // Try to decode one frame.
                match video_decoder.take() {
                    Ok(Some(frame)) => {
                        // Process frame.
                        result = Ok(Some(Self::process_video_frame(frame)?));
                        break;
                    }
                    Ok(None) => {
                        // Retry only once.
                        if retried {
                            break;
                        }
                        // Demux next packet.
                        if let Some(demuxer) = &mut self.demuxer {
                            while let Some(packet) = demuxer.take()? {
                                if packet.stream_index() == self.video_stream_index {
                                    video_decoder.push(packet)?;
                                    break;
                                }
                            }
                        }
                        // Retry.
                        retried = true;
                    }
                    Err(err) => {
                        result = Err(err);
                        break;
                    }
                }
            }
        }
        result
    }

    #[cfg(feature = "video")]
    fn process_video_frame(video_frame: VideoFrame) -> Result<YuvBuffer, Error> {
        let width = video_frame.width();
        let height = video_frame.height();
        let half_width = width / 2;
        let half_height = height / 2;

        let mut pixels_y = vec![0; width * height].into_boxed_slice();
        let mut pixels_u = vec![0; half_width * half_height].into_boxed_slice();
        let mut pixels_v = vec![0; half_width * half_height].into_boxed_slice();

        // Copy planes without line-padding for alignment.
        //let [plane0, plane1, plane2, plane3] = *video_frame.planes();
        for (y, line0) in video_frame.planes()[0].lines().enumerate() {
            let start = y * width;
            let end = (y + 1) * width;
            pixels_y[start..end].copy_from_slice(&line0[..width]);
        }
        for (y, line1) in video_frame.planes()[1].lines().enumerate() {
            let start = y * half_width;
            let end = (y + 1) * half_width;
            pixels_u[start..end].copy_from_slice(&line1[..half_width]);
        }
        for (y, line2) in video_frame.planes()[2].lines().enumerate() {
            let start = y * half_width;
            let end = (y + 1) * half_width;
            pixels_v[start..end].copy_from_slice(&line2[..half_width]);
        }

        Ok(YuvBuffer {
            pixels_y,
            pixels_u,
            pixels_v,
            width: width as u32,
            height: height as u32,
        })
    }
}

impl Node for UploadVideo {
    fn new(window: &Window) -> Self {
        Self {
            uploader: UploadYuvBuffer::new(window),
            #[cfg(feature = "video")]
            demuxer: None,
            #[cfg(feature = "video")]
            video_stream_index: 0,
            #[cfg(feature = "video")]
            video_decoder: None,
        }
    }

    fn update_io(
        &mut self,
        window: &Window,
        source: (Option<DeviceSource>, Option<DeviceTarget>),
        target_candidate: (Option<DeviceSource>, Option<DeviceTarget>),
    ) -> (Option<DeviceSource>, Option<DeviceTarget>) {
        self.uploader.update_io(window, source, target_candidate)
    }

    fn render(&mut self, window: &Window) {
        #[cfg(feature = "video")]
        if let Ok(Some(buffer)) = self.next_frame() {
            self.uploader.enqueue_buffer(buffer);
        }
        self.uploader.render(window)
    }
}
