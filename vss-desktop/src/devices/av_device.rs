use std::cell::RefCell;
use std::cell::UnsafeCell;
use std::fs::File;

use av;
use av::codec::Codec;
use av::common::Ts;
use av::ffi::AVCodecID::*;
use av::ffi::AVPixelFormat::*;
use av::format::Demuxer;
use av::format::{Muxer, OutputFormat};
use av::generic::Encoder;
use av::generic::{Decoder, Frame, Frames};
use av::video;

use vss::*;

struct Output {
    muxer: RefCell<Muxer>,
    encoders: RefCell<Vec<Encoder>>,
    timestamps: RefCell<Vec<Ts>>,
    frames_out: RefCell<Vec<Frame>>,
}

pub struct AvDevice {
    video: VideoDevice,

    demuxer: RefCell<Demuxer>,
    decoders: UnsafeCell<Vec<Decoder>>,
    frames_in: RefCell<Option<Frames<'static>>>,

    output: Option<Output>,
}

impl AvDevice {
    pub fn new(config: &Config) -> Self {
        av::LibAV::init();

        let (demuxer, decoders) = Self::new_input(&config.input);

        let output = if !config.output.is_empty() {
            println!("[video] streaming to {}", config.output);
            let (muxer, encoders, timestamps, frames_out) = Self::new_output(&config.output);
            Some(Output {
                muxer: RefCell::new(muxer),
                encoders: RefCell::new(encoders),
                timestamps: RefCell::new(timestamps),
                frames_out: RefCell::new(frames_out),
            })
        } else {
            None
        };

        AvDevice {
            video: VideoDevice::new(config),
            demuxer: RefCell::new(demuxer),
            decoders: UnsafeCell::new(decoders),
            frames_in: RefCell::new(None),
            output,
        }
    }

    fn new_input(filename: &str) -> (Demuxer, Vec<Decoder>) {
        // Open input file.
        let file = File::open(filename).expect("Failed to open input video");

        // Initialize demuxer.
        let demuxer = Demuxer::open(file).unwrap();
        if cfg!(debug_assertions) {
            demuxer.dump_info();
        }

        // Initialize decoders.
        let decoders = demuxer
            .streams()
            .map(|stream| Decoder::from_stream(&stream))
            .collect::<av::Result<Vec<Decoder>>>()
            .unwrap();

        (demuxer, decoders)
    }

    fn new_output(filename: &str) -> (Muxer, Vec<Encoder>, Vec<Ts>, Vec<Frame>) {
        let width = 1024;
        let height = 768;
        let framerate = 30;
        let align = 32;

        let file = File::create(filename).expect("Failed to create output video");

        // Create video encoder
        let output_format = OutputFormat::from_name("mp4").expect("Output format not found");
        let video_codec = Codec::find_encoder_by_id(AV_CODEC_ID_H264).unwrap();
        let video_encoder = video::Encoder::from_codec(video_codec)
            .unwrap()
            .width(width)
            .height(height)
            .pixel_format(*video_codec.pixel_formats().first().unwrap())
            .time_base(framerate)
            .open(output_format)
            .expect("Failed to open encoder");

        // Setup a base frame.
        let mut timestamps = Vec::<Ts>::new();
        let mut encoders = Vec::<Encoder>::new();
        let mut frames = Vec::<Frame>::new();
        frames.push(
            video::Frame::new(width, height, AV_PIX_FMT_RGB24, align)
                .unwrap()
                .into(),
        );
        timestamps.push(Ts::new(0, video_encoder.time_base()));
        encoders.push(video_encoder.into());

        // Create a muxer.
        let mut muxer = Muxer::new(output_format, file).unwrap();
        for encoder in &encoders {
            muxer.add_stream_from_encoder(&encoder).unwrap();
        }
        let muxer = muxer.open().unwrap();
        muxer.dump_info();

        (muxer, encoders, timestamps, frames)
    }

    fn read_frame(&self) {
        let mut frames_in = self.frames_in.borrow_mut();

        // Test if we have to decode new frames.
        if frames_in.is_none() {
            // Demux next packet.
            let mut demuxer = self.demuxer.borrow_mut();
            if let Some(packet) = demuxer.read_packet().unwrap() {
                // Find the correct decoder for that packet.
                let decoders = self.decoders.get();
                // Undermine borrowing rules, because we know.
                let decoder = unsafe { &mut (*decoders)[packet.stream_index()] };
                // Decode packet into frames.
                frames_in.replace(decoder.decode(packet).unwrap());
            };
        }

        // Pull next frame, if available.
        if let Some(ref mut frame_iter) = *frames_in {
            match frame_iter.next() {
                Some(Ok(Frame::Audio(_))) => {}
                Some(Ok(Frame::Video(video_frame))) => {
                    // Create a copy of the frame with resolution-sized width, i.e., without padding for alignment.
                    let width = video_frame.width();
                    let height = video_frame.height();
                    let half_width = width / 2;
                    let half_height = height / 2;
                    let linesize0 = video_frame.linesize(0);
                    let linesize1 = video_frame.linesize(1);
                    let linesize2 = video_frame.linesize(2);
                    let data = &video_frame.data();
                    let mut pixels_y = vec![0; width * height].into_boxed_slice();
                    let mut pixels_u = vec![0; half_width * half_height].into_boxed_slice();
                    let mut pixels_v = vec![0; half_width * half_height].into_boxed_slice();
                    for y in 0..height {
                        for x in 0..width {
                            pixels_y[y * width + x] = data[0][y * linesize0 + x];
                        }
                    }
                    for y in 0..half_height {
                        for x in 0..half_width {
                            pixels_u[y * half_width + x] = data[1][y * linesize1 + x];
                            pixels_v[y * half_width + x] = data[2][y * linesize2 + x];
                        }
                    }

                    self.video.upload_yuv(YUVBuffer {
                        pixels_y,
                        pixels_u,
                        pixels_v,
                        width,
                        height,
                    });
                }
                Some(Err(_)) => {}
                None => {
                    frames_in.take();
                }
            }
        }
    }

    fn write_frame(&self) {
        let output = self.output.as_ref().unwrap();
        let mut muxer = output.muxer.borrow_mut();
        let mut encoders = output.encoders.borrow_mut();
        let mut timestamps = output.timestamps.borrow_mut();
        let mut frames_out = output.frames_out.borrow_mut();

        let index = timestamps
            .iter()
            .enumerate()
            .min_by_key(|&(_, ts)| ts)
            .unwrap()
            .0;
        let ts = &mut timestamps[index];
        let encoder = &mut encoders[index];

        if let Encoder::Video(ref mut encoder) = *encoder {
            let video_frame = frames_out[index].as_mut_video_frame().unwrap();

            video_frame.set_pts(ts.index());
            let width = video_frame.width();
            let height = video_frame.height();
            let linesize = video_frame.linesize(0);
            let mut buffer = vec![0u8; linesize * height * 3].into_boxed_slice();

            // Copy to aligned video buffer.
            let rgb_data = self.video.download_rgb();
            for y in 0..height.min(rgb_data.height) {
                for x in 0..width.min(rgb_data.width) {
                    buffer[(y * linesize + x) * 3 + 0] =
                        rgb_data.pixels_rgb[(y * width + x) * 3 + 0];
                    buffer[(y * linesize + x) * 3 + 1] =
                        rgb_data.pixels_rgb[(y * width + x) * 3 + 1];
                    buffer[(y * linesize + x) * 3 + 2] =
                        rgb_data.pixels_rgb[(y * width + x) * 3 + 2];
                }
            }

            video_frame.fill_channel(0, &buffer).unwrap();
            *ts += 1;

            // Encode and mux video frame
            let encoded_frame = encoder.encode(video_frame).unwrap();
            muxer.mux_all(encoded_frame, index).unwrap();
        }
    }
}

impl Drop for AvDevice {
    fn drop(&mut self) {
        if let Some(ref output) = self.output {
            let mut muxer = output.muxer.borrow_mut();
            let encoders = output.encoders.replace(Vec::new());
            for (index, encoder) in encoders.into_iter().enumerate() {
                muxer.mux_all(encoder.flush().unwrap(), index).unwrap();
            }
        }
    }
}

impl Device for AvDevice {
    fn factory(&self) -> &RefCell<DeviceFactory> {
        self.video.factory()
    }

    fn encoder(&self) -> &RefCell<DeviceEncoder> {
        self.video.encoder()
    }

    fn gaze(&self) -> DeviceGaze {
        self.video.gaze()
    }

    fn source(&self) -> &RefCell<DeviceSource> {
        self.video.source()
    }

    fn target(&self) -> &RefCell<DeviceTarget> {
        self.video.target()
    }

    fn begin_frame(&self) {
        self.video.begin_frame();
        self.read_frame();
    }

    fn end_frame(&self, done: &mut bool) {
        self.video.end_frame(done);
        if self.output.is_some() {
            self.write_frame();
            *done = true;
        }
    }
}
