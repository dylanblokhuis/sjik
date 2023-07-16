use std::collections::{HashMap, VecDeque};
use std::slice;
use std::sync::{Arc, Mutex};

use cpal::{traits::StreamTrait, ChannelCount, SampleRate, Stream};
use ringbuf::{HeapConsumer, HeapProducer, HeapRb};
use stainless_ffmpeg::prelude::FormatContext;
use stainless_ffmpeg::prelude::*;
use stainless_ffmpeg::probe::Probe;
// Since the Rust time-functions `Duration` and `Instant` work with nanoseconds
// by default, it is much simpler to convert a PTS to nanoseconds,
// that is why the following constant has been added.
const ONE_NANOSECOND: i64 = 1000000000;

pub struct MediaDecoder {
    audio_producer: HeapProducer<f32>,
    video_queue: Arc<Mutex<VecDeque<(i64, Vec<u8>)>>>,
    _audio_stream: Stream,
    format_context: FormatContext,
    audio_decoder: AudioDecoder,
    video_decoder: VideoDecoder,
    audio_graph: FilterGraph,
    video_graph: FilterGraph,
    audio_stream_index: isize,
    video_stream_index: isize,
}

impl MediaDecoder {
    pub fn new<F>(path_or_url: &str, new_frame_callback: F) -> Self
    where
        F: Fn(Vec<u8>) + Send + Sync + 'static,
    {
        let mut probe = Probe::new(path_or_url);
        probe.process(log::LevelFilter::Off).unwrap();
        println!("{}", probe.format.unwrap());

        let mut format_context = FormatContext::new(path_or_url).unwrap();
        format_context.open_input().unwrap();

        let mut first_audio_stream = None;
        let mut first_video_stream = None;
        for i in 0..format_context.get_nb_streams() {
            let stream_type = format_context.get_stream_type(i as isize);
            println!("Stream {}: {:?}", i, stream_type);

            if stream_type == AVMediaType::AVMEDIA_TYPE_AUDIO {
                first_audio_stream = Some(i as isize);
            }
            if stream_type == AVMediaType::AVMEDIA_TYPE_VIDEO {
                first_video_stream = Some(i as isize);
            }
        }

        let first_audio_stream = first_audio_stream.unwrap();
        let first_video_stream = first_video_stream.unwrap();

        let audio_decoder = AudioDecoder::new(
            "audio_decoder".to_string(),
            &format_context,
            first_audio_stream,
        )
        .unwrap();

        let video_decoder = VideoDecoder::new(
            "video_decoder".to_string(),
            &format_context,
            first_video_stream,
        )
        .unwrap();

        let mut audio_graph = FilterGraph::new().unwrap();

        let resample_rate = 48000;
        let channels = 2;

        let video_graph = {
            let mut video_graph = FilterGraph::new().unwrap();
            video_graph
                .add_input_from_video_decoder("source_video", &video_decoder)
                .unwrap();

            let format_filter = {
                let mut parameters = HashMap::new();

                parameters.insert(
                    "pix_fmts".to_string(),
                    // yuv420p, yuv444p, yuv422p, yuv420p10le, yuv444p10le, yuv422p10le
                    ParameterValue::String("yuv420p".to_string()),
                );

                let filter = Filter {
                    name: "format".to_string(),
                    label: Some("Format video".to_string()),
                    parameters,
                    inputs: None,
                    outputs: None,
                };

                video_graph.add_filter(&filter).unwrap()
            };

            video_graph.add_video_output("main_video").unwrap();

            video_graph
                .connect_input("source_video", 0, &format_filter, 0)
                .unwrap();
            video_graph
                .connect_output(&format_filter, 0, "main_video", 0)
                .unwrap();
            video_graph.validate().unwrap();

            video_graph
        };

        //  audio graph
        let audio_graph = {
            audio_graph
                .add_input_from_audio_decoder("source_audio", &audio_decoder)
                .unwrap();

            let mut parameters = HashMap::new();
            parameters.insert(
                "sample_rates".to_string(),
                ParameterValue::String(resample_rate.to_string()),
            );
            parameters.insert(
                "channel_layouts".to_string(),
                ParameterValue::String(if channels == 1 {
                    "mono".to_string()
                } else {
                    "stereo".to_string()
                }),
            );
            parameters.insert(
                "sample_fmts".to_string(),
                ParameterValue::String("s32".to_string()),
            );

            let filter = Filter {
                name: "aformat".to_string(),
                label: Some("Format audio samples".to_string()),
                parameters,
                inputs: None,
                outputs: None,
            };

            let filter = audio_graph.add_filter(&filter).unwrap();
            audio_graph.add_audio_output("main_audio").unwrap();

            audio_graph
                .connect_input("source_audio", 0, &filter, 0)
                .unwrap();
            audio_graph
                .connect_output(&filter, 0, "main_audio", 0)
                .unwrap();
            audio_graph.validate().unwrap();

            audio_graph
        };

        let video_queue = Arc::new(Mutex::new(VecDeque::with_capacity(10)));
        let (audio_producer, audio_consumer) = HeapRb::<f32>::new(50 * 1024 * 1024).split();

        let video_queue_clone = video_queue.clone();
        std::thread::spawn(move || {
            let mut prev_pts = None;
            let mut now = std::time::Instant::now();

            loop {
                // std::thread::sleep(std::time::Duration::from_millis(10));
                // log::debug!(
                //     "video queue size: {}",
                //     video_queue_clone.lock().unwrap().len()
                // );
                if let Some((pts, frame)) = video_queue_clone.lock().unwrap().pop_back() {
                    if let Some(prev) = prev_pts {
                        let elapsed = now.elapsed();
                        if pts > prev {
                            let sleep_time = std::time::Duration::new(0, (pts - prev) as u32);
                            if elapsed < sleep_time {
                                log::debug!("sleeping for {:?}", sleep_time - elapsed);
                                spin_sleep::sleep(sleep_time - elapsed);
                            }
                        }
                    }

                    prev_pts = Some(pts);
                    now = std::time::Instant::now();

                    new_frame_callback(frame);
                }
            }
        });

        let _audio_stream =
            setup_audio_stream(audio_consumer, channels, SampleRate(resample_rate as u32));
        _audio_stream.play().unwrap();

        Self {
            audio_producer,
            video_queue,
            _audio_stream,
            format_context,
            audio_decoder,
            video_decoder,
            audio_graph,
            video_graph,
            video_stream_index: first_video_stream,
            audio_stream_index: first_audio_stream,
        }
    }

    pub fn get_video_size(&self) -> (u32, u32) {
        let width = self.video_decoder.get_width() as u32;
        let height = self.video_decoder.get_height() as u32;
        (width, height)
    }

    pub fn start(&mut self) {
        loop {
            if self.video_queue.lock().unwrap().len() >= 10 {
                std::thread::sleep(std::time::Duration::from_millis(10));
                continue;
            }

            let Ok(packet) = self.format_context.next_packet() else {
                break;
            };

            if packet.get_stream_index() == self.video_stream_index {
                let Ok(frame) = self.video_decoder.decode(&packet) else {
                    continue;
                };

                unsafe {
                    let bet: i64 = (*frame.frame).best_effort_timestamp;

                    let sw_frame = av_frame_alloc();
                    let tmp_frame = if (*frame.frame).format
                        == self.video_decoder.hw_pixel_format.unwrap() as i32
                    {
                        av_hwframe_transfer_data(sw_frame, frame.frame, 0);
                        sw_frame
                    } else {
                        frame.frame
                    };

                    let (_, frames) = self
                        .video_graph
                        .process(
                            &[],
                            &[Frame {
                                frame: tmp_frame,
                                index: self.video_stream_index as usize,
                                name: None,
                            }],
                        )
                        .unwrap();
                    let frame = frames.first().unwrap();

                    let stream =
                        (*self.format_context.get_stream(self.video_stream_index)).time_base;
                    let pts_nano = av_rescale_q(bet, stream, av_make_q(1, ONE_NANOSECOND as i32));

                    let frame = *frame.frame;
                    log::debug!("ln 0 {:?}", frame.linesize[0]);
                    log::debug!("ln 1 {:?}", frame.linesize[1]);
                    log::debug!("ln 2 {:?}", frame.linesize[2]);
                    log::debug!("ln 3 {:?}", frame.linesize[3]);
                    log::debug!("ln 4 {:?}", frame.linesize[4]);

                    log::debug!("d 0 {:?}", frame.data[0]);
                    log::debug!("d 1 {:?}", frame.data[1]);
                    log::debug!("d 2 {:?}", frame.data[2]);
                    log::debug!("d 3 {:?}", frame.data[3]);
                    log::debug!("d 4 {:?}", frame.data[4]);

                    let color_data = match self.video_decoder.get_pix_fmt_name().as_str() {
                        "yuv420p" => self.frame_to_yuv420_3_plane(frame),
                        "yuv420p10le" => self.frame_to_yuv420_3_plane(frame),
                        "dxva2_vld" => self.frame_to_yuv420_3_plane(frame),
                        "cuda" => self.frame_to_yuv420_3_plane(frame),
                        "videotoolbox_vld" => self.frame_to_yuv420_3_plane(frame),
                        x => panic!("Unsupported pixel format {x}"),
                    };
                    log::debug!("color_data {:?}", color_data.len());
                    self.video_queue
                        .lock()
                        .unwrap()
                        .push_front((pts_nano, color_data));
                }
            }

            if packet.get_stream_index() == self.audio_stream_index {
                let Ok(frame) = self.audio_decoder.decode(&packet) else {
                    continue;
                };
                let (frames, _) = self.audio_graph.process(&[frame], &[]).unwrap();
                let frame = frames.first().unwrap();

                unsafe {
                    let frame = frame.frame;
                    let size = ((*frame).channels * (*frame).nb_samples) as usize;
                    let data: Vec<i32> =
                        slice::from_raw_parts((*frame).data[0] as _, size).to_vec();
                    let float_samples: Vec<f32> = data
                        .iter()
                        .map(|value| (*value as f32) / i32::MAX as f32)
                        .collect();

                    self.audio_producer.push_slice(&float_samples);
                }
            }
        }
    }

    pub unsafe fn frame_to_yuv420_3_plane(&self, frame: AVFrame) -> Vec<u8> {
        let height = frame.height;

        let y_plane_size = (frame.linesize[0] * height) as usize;
        let u_plane_size = (frame.linesize[1] * (height / 2)) as usize;
        let v_plane_size = (frame.linesize[2] * (height / 2)) as usize;

        log::debug!("y size {:?}", y_plane_size);
        log::debug!("u size {:?}", u_plane_size);
        log::debug!("v size {:?}", v_plane_size);

        let mut vec = vec![0; y_plane_size + u_plane_size + v_plane_size];

        let y = slice::from_raw_parts(frame.data[0], y_plane_size);
        let u = slice::from_raw_parts(frame.data[1], u_plane_size);
        let v = slice::from_raw_parts(frame.data[2], v_plane_size);

        vec[..y_plane_size].copy_from_slice(y);
        vec[y_plane_size..(y_plane_size + u_plane_size)].copy_from_slice(u);
        vec[(y_plane_size + u_plane_size)..].copy_from_slice(v);

        vec
    }

    pub unsafe fn frame_to_yuv420_2_plane(&self, frame: AVFrame) -> Vec<u8> {
        let height = frame.height;

        let y_plane_size = (frame.linesize[0] * height) as usize;
        let uv_plane_size = (frame.linesize[1] * (height / 2)) as usize;

        log::debug!("y size {:?}", y_plane_size);
        log::debug!("uv size {:?}", uv_plane_size);

        let mut vec = vec![0; y_plane_size + uv_plane_size];

        let y = slice::from_raw_parts(frame.data[0], y_plane_size);
        let uv = slice::from_raw_parts(frame.data[1], uv_plane_size);

        vec[..y_plane_size].copy_from_slice(y);
        vec[y_plane_size..].copy_from_slice(uv);

        vec
    }

    pub unsafe fn frame_to_yuv420_101e(&self, frame: AVFrame) -> Vec<u8> {
        let width = frame.width;
        let height = frame.height;

        let size = (width * height) as usize;

        log::debug!("y size {:?}", size);
        log::debug!("u size {:?}", size / 2);
        log::debug!("v size {:?}", size / 2);
        let mut vec = vec![0; size + (size / 2) + (size / 2)];
        let y = slice::from_raw_parts(frame.data[0], size);
        let u = slice::from_raw_parts(frame.data[1], size / 2);
        let v = slice::from_raw_parts(frame.data[2], size / 2);

        log::debug!("copy y");
        vec[..size].copy_from_slice(y);
        log::debug!("copy u");
        vec[size..(size + size / 2)].copy_from_slice(u);
        log::debug!("copy v");
        vec[(size + size / 2)..].copy_from_slice(v);
        log::debug!("done");
        vec
    }
}

fn setup_audio_stream(
    mut audio_consumer: HeapConsumer<f32>,
    channels: ChannelCount,
    sample_rate: SampleRate,
) -> Stream {
    use cpal::traits::{DeviceTrait, HostTrait};

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("no output device available");

    let mut supported_configs_range = device
        .supported_output_configs()
        .expect("error while querying configs");

    let supported_config = supported_configs_range
        .find(|config| {
            config.channels() == channels
                && sample_rate >= config.min_sample_rate()
                && sample_rate <= config.max_sample_rate()
                && config.sample_format() == cpal::SampleFormat::F32
        })
        .expect("no supported config?!")
        .with_sample_rate(sample_rate);

    let config = supported_config.into();

    device
        .build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                audio_consumer.pop_slice(data);
            },
            move |err| println!("CPAL error: {:?}", err),
            None,
        )
        .unwrap()
}
