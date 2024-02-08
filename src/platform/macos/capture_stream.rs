use crate::{capture_stream::{CaptureConfig, StreamCreateError, StreamError, StreamEvent}, prelude::{AudioCaptureConfig, Capturable, VideoFrame, AudioFrame}, util::{Rect, Size}};
use super::{frame::{MacosAudioFrame, MacosVideoFrame}, objc_wrap::{CGPoint, CGRect, CGSize, CMTime, NSArray, SCContentFilter, SCStream, SCStreamCallbackError, SCStreamConfiguration, SCStreamDelegate, SCStreamOutputType}};

pub(crate) struct MacosCaptureStream {
    _stream: SCStream
}

pub trait MacosCaptureConfigExt {
    fn with_output_size(self, size: Size) -> Self;
    fn with_scale_to_fit(self, scale_to_fit: bool) -> Self;
    fn with_preserve_aspect_ratio(self, preserve_aspect_ratio: bool) -> Self;
    fn with_maximum_fps(self, maximum_fps: Option<f32>) -> Self;
    fn with_queue_depth(self, queue_depth: usize) -> Self;
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct MacosCaptureConfig {
    output_size: Size,
    scale_to_fit: bool,
    preserve_aspect_ratio: bool,
    maximum_fps: Option<f32>,
    queue_depth: usize,
}

impl MacosCaptureConfig {
    pub fn new(source_rect: Rect) -> Self {
        Self {
            output_size: source_rect.size,
            scale_to_fit: true,
            preserve_aspect_ratio: false,
            maximum_fps: None,
            queue_depth: 4,
        }
    }
}

impl MacosCaptureConfigExt for CaptureConfig {
    fn with_output_size(self, output_size: Size) -> Self {
        Self {
            impl_capture_config: MacosCaptureConfig {
                output_size,
                ..self.impl_capture_config
            },
            ..self
        }
    }

    fn with_scale_to_fit(self, scale_to_fit: bool) -> Self {
        Self {
            impl_capture_config: MacosCaptureConfig {
                scale_to_fit,
                ..self.impl_capture_config
            },
            ..self
        }
    }

    fn with_preserve_aspect_ratio(self, preserve_aspect_ratio: bool) -> Self {
        Self {
            impl_capture_config: MacosCaptureConfig {
                preserve_aspect_ratio,
                ..self.impl_capture_config
            },
            ..self
        }
    }

    fn with_maximum_fps(self, maximum_fps: Option<f32>) -> Self {
        Self {
            impl_capture_config: MacosCaptureConfig {
                maximum_fps,
                ..self.impl_capture_config
            },
            ..self
        }
    }

    fn with_queue_depth(self, queue_depth: usize) -> Self {
        Self {
            impl_capture_config: MacosCaptureConfig {
                queue_depth,
                ..self.impl_capture_config
            },
            ..self
        }
    }
}

pub trait MacosAudioCaptureConfigExt {
    fn set_exclude_current_process_audio(self, exclude_current_process_audio: bool) -> Self;
}

#[derive(Copy, Clone, Debug)]
pub(crate) struct MacosAudioCaptureConfig {
    exclude_current_process_audio: bool,
}

impl MacosAudioCaptureConfig {
    pub fn new() -> Self {
        Self {
            exclude_current_process_audio: false,
        }
    }
}

impl MacosAudioCaptureConfigExt for AudioCaptureConfig {
    fn set_exclude_current_process_audio(self, exclude_current_process_audio: bool) -> Self {
        Self {
            impl_capture_audio_config: MacosAudioCaptureConfig {
                exclude_current_process_audio,
                ..self.impl_capture_audio_config
            },
            ..self
        }
    }
}

impl MacosCaptureStream {
    pub fn new(config: CaptureConfig, mut callback: Box<impl FnMut(Result<StreamEvent, StreamError>) + Send + 'static>) -> Result<Self, StreamCreateError> {
        let content_filter = match config.target {
            Capturable::Window(window) => SCContentFilter::new_with_desktop_independent_window(window.impl_capturable_window.window.clone()),
            Capturable::Display(display) => SCContentFilter::new_with_display_excluding_windows(display.impl_capturable_display.display.clone(), NSArray::new())
        };

        let mut stream_configuration = SCStreamConfiguration::new();
        stream_configuration.set_background_color(super::objc_wrap::SCStreamBackgroundColor::Black);
        stream_configuration.set_pixel_format(super::objc_wrap::SCStreamPixelFormat::V420);
        stream_configuration.set_queue_depth(3);
        stream_configuration.set_scales_to_fit(config.impl_capture_config.scale_to_fit);
        stream_configuration.set_source_rect(CGRect {
            origin: CGPoint {
                x: config.source_rect.origin.x,
                y: config.source_rect.origin.y,
            },
            size: CGSize {
                x: config.source_rect.size.width,
                y: config.source_rect.size.height
            }
        });
        stream_configuration.set_size(CGSize {
            x: config.impl_capture_config.output_size.width,
            y: config.impl_capture_config.output_size.height
        });
        stream_configuration.set_show_cursor(config.show_cursor);
        let min_frame_interval = if let Some(max_fps) = config.impl_capture_config.maximum_fps {
            1.0 / max_fps
        } else {
            0.0
        };
        stream_configuration.set_minimum_time_interval(CMTime::new_with_seconds(min_frame_interval as f64, 10000));
        stream_configuration.set_queue_depth(config.impl_capture_config.queue_depth as isize);
        stream_configuration.set_preserves_aspect_ratio(config.impl_capture_config.preserve_aspect_ratio);
        if let Some(audio_config) = config.capture_audio {
            stream_configuration.set_capture_audio(true);
            let sample_rate = match audio_config.sample_rate {
                crate::prelude::SampleRate::Hz8000 => super::objc_wrap::SCStreamSampleRate::R8000,
                crate::prelude::SampleRate::Hz16000 => super::objc_wrap::SCStreamSampleRate::R16000,
                crate::prelude::SampleRate::Hz24000 => super::objc_wrap::SCStreamSampleRate::R24000,
                crate::prelude::SampleRate::Hz48000 => super::objc_wrap::SCStreamSampleRate::R48000,
            };
            stream_configuration.set_sample_rate(sample_rate);
            let channel_count = match audio_config.channel_count {
                crate::prelude::ChannelCount::Mono => 1,
                crate::prelude::ChannelCount::Stereo => 2,
            };
            stream_configuration.set_channel_count(channel_count);
            stream_configuration.set_exclude_current_process_audio(audio_config.impl_capture_audio_config.exclude_current_process_audio);
        } else {
            stream_configuration.set_capture_audio(false);
        };

        let mut video_frame_counter = 0u64;
        let mut audio_frame_counter = 0u64;
        let delegate = SCStreamDelegate::new(move |result| {
            match result {
                Ok((sample_buffer, output_type)) => {
                    match output_type {
                        SCStreamOutputType::Screen => {
                            let frame_id = video_frame_counter;
                            video_frame_counter += 1;
                            let video_frame = MacosVideoFrame {
                                sample_buffer,
                                frame_id
                            };
                            (callback)(
                                Ok(
                                    StreamEvent::Video(
                                        VideoFrame {
                                            impl_video_frame: video_frame
                                        }
                                    )
                                )
                            );
                        },
                        SCStreamOutputType::Audio => {
                            let frame_id = audio_frame_counter;
                            audio_frame_counter += 1;
                            let audio_frame = MacosAudioFrame {
                                sample_buffer,
                                frame_id
                            };
                            (callback)(
                                Ok(
                                    StreamEvent::Audio(
                                        AudioFrame {
                                            impl_audio_frame: audio_frame
                                        }
                                    )
                                )
                            );
                        }
                    }
                },
                Err(SCStreamCallbackError::StreamStopped) => {
                    (callback)(
                        Ok(
                            StreamEvent::End
                        )
                    );
                },
                Err(error) => {
                    (callback)(
                        Err(StreamError::Other(match error {
                            SCStreamCallbackError::SampleBufferCopyFailed => "Failed to copy sample buffer".into(),
                            SCStreamCallbackError::Other(error) => format!("System error: {}", error.code()),
                            SCStreamCallbackError::StreamStopped => unreachable!()
                        }))
                    );
                }
            }
        });

        let mut stream = SCStream::new(content_filter, stream_configuration, delegate)?;
        stream.start()?;

        Ok(Self {
            _stream: stream
        })

    }
}

impl Drop for MacosCaptureStream {
    fn drop(&mut self) {
    }
}
