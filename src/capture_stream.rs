use std::{error::Error, fmt::Display};

use crate::platform::platform_impl::{ImplAudioCaptureConfig, ImplAudioFrame, ImplCaptureConfig, ImplCaptureStream, ImplVideoFrame};
use crate::capturable_content::Capturable;
use crate::prelude::{CapturableDisplay, CapturableWindow};
use crate::util::{Point, Rect, Size};

pub struct AudioFrame {
    pub(crate) impl_audio_frame: ImplAudioFrame,
}

pub struct VideoFrame {
    pub(crate) impl_video_frame: ImplVideoFrame,
}

pub enum StreamEvent {
    Audio(AudioFrame),
    Video(VideoFrame),
    Idle,
    End,
}

#[derive(Debug, Clone)]
pub enum StreamError {
    Other(String),
}

impl Display for StreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(message) => f.write_fmt(format_args!("StreamError::Other(\"{}\")", message))
        }
    }
}

impl Error for StreamError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

pub struct CaptureStream {
    _impl_capture_stream: ImplCaptureStream,
}

#[derive(Debug, Clone)]
pub enum StreamCreateError {
    Other(String)
    //GpuLost,
}

impl Display for StreamCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(message) => f.write_fmt(format_args!("StreamCreateError::Other(\"{}\")", message))
        }
    }
}

impl Error for StreamCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

#[derive(Copy, Clone, Debug)]
pub enum SampleRate {
    Hz8000,
    Hz16000,
    Hz24000,
    Hz48000,
}

#[derive(Copy, Clone, Debug)]
pub enum ChannelCount {
    Mono,
    Stereo
}

#[derive(Copy, Clone, Debug)]
pub struct AudioCaptureConfig {
    pub(crate)  sample_rate: SampleRate,
    pub(crate)  channel_count: ChannelCount,
    pub(crate)  impl_capture_audio_config: ImplAudioCaptureConfig,
}

impl AudioCaptureConfig {
    pub fn new() -> Self {
        Self {
            sample_rate: SampleRate::Hz24000,
            channel_count: ChannelCount::Mono,
            impl_capture_audio_config: ImplAudioCaptureConfig::new()
        }
    }
}

#[derive(Clone, Debug)]
pub struct CaptureConfig {
    pub(crate) target: Capturable,
    pub(crate) source_rect: Rect,
    pub(crate) show_cursor: bool,
    pub(crate) capture_audio: Option<AudioCaptureConfig>,
    pub(crate) impl_capture_config: ImplCaptureConfig,
}

impl CaptureConfig {
    pub fn with_window(window: CapturableWindow) -> CaptureConfig {
        let rect = window.rect();
        CaptureConfig {
            target: Capturable::Window(window),
            source_rect: Rect {
                origin: Point {
                    x: 0.0,
                    y: 0.0,
                },
                size: rect.size
            },
            show_cursor: false,
            impl_capture_config: ImplCaptureConfig::new(rect),
            capture_audio: None,
        }
    }

    pub fn with_display(display: CapturableDisplay) -> CaptureConfig {
        let rect = display.rect();
        CaptureConfig {
            target: Capturable::Display(display),
            source_rect: Rect {
                origin: Point {
                    x: 0.0,
                    y: 0.0,
                },
                size: rect.size
            },
            show_cursor: false,
            impl_capture_config: ImplCaptureConfig::new(rect),
            capture_audio: None,
        }
    }
}

impl CaptureStream {
    pub fn new(config: CaptureConfig, callback: impl FnMut(Result<StreamEvent, StreamError>) + Send + 'static) -> Result<Self, StreamCreateError> {
        let boxed_callback = Box::new(callback);
        Ok(Self {
            _impl_capture_stream: ImplCaptureStream::new(config, boxed_callback)?
        })
    }
}


