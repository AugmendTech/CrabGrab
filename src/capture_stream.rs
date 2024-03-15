use std::fmt::Debug;
use std::{error::Error, fmt::Display};

use crate::platform::platform_impl::{ImplAudioCaptureConfig, ImplAudioFrame, ImplCaptureConfig, ImplCaptureStream, ImplVideoFrame, ImplPixelFormat};
use crate::capturable_content::Capturable;
use crate::prelude::{AudioChannelCount, AudioFrame, AudioSampleRate, CapturableDisplay, CapturableWindow, VideoCaptureFrame, VideoFrame};
use crate::util::{Point, Rect, Size};

#[derive(Debug)]
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

#[derive(Debug, Clone)]
pub enum StreamCreateError {
    Other(String),
    UnsupportedPixelFormat,
    //GpuLost,
}

unsafe impl Send for StreamCreateError {}
unsafe impl Sync for StreamCreateError {}

#[derive(Debug)]
pub enum StreamStopError {
    Other(String),
    AlreadyStopped,
    //GpuLost,
}

unsafe impl Send for StreamStopError {}
unsafe impl Sync for StreamStopError {}

impl Display for StreamCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(message) => f.write_fmt(format_args!("StreamCreateError::Other(\"{}\")", message)),
            Self::UnsupportedPixelFormat => f.write_fmt(format_args!("SteamCreateError::UnsupportedPixelFormat")),
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

#[derive(Clone, Debug)]
pub struct AudioCaptureConfig {
    pub(crate)  sample_rate: AudioSampleRate,
    pub(crate)  channel_count: AudioChannelCount,
    pub(crate)  impl_capture_audio_config: ImplAudioCaptureConfig,
}

impl AudioCaptureConfig {
    pub fn new() -> Self {
        Self {
            sample_rate: AudioSampleRate::Hz24000,
            channel_count: AudioChannelCount::Mono,
            impl_capture_audio_config: ImplAudioCaptureConfig::new()
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CapturePixelFormat {
    Bgra8888,
    Argb2101010,
    V420,
    F420,
}

#[derive(Clone, Debug)]
pub struct CaptureConfig {
    pub(crate) target: Capturable,
    pub(crate) source_rect: Rect,
    pub(crate) output_size: Size,
    pub(crate) show_cursor: bool,
    pub(crate) pixel_format: CapturePixelFormat,
    pub(crate) capture_audio: Option<AudioCaptureConfig>,
    pub(crate) impl_capture_config: ImplCaptureConfig,
    pub(crate) buffer_count: usize,
}

#[derive(Debug, Clone)]
pub enum CaptureConfigError {
    UnsupportedPixelFormat,
    InvalidBufferCount,
}

impl CaptureConfig {
    pub fn with_window(window: CapturableWindow, pixel_format: CapturePixelFormat) -> Result<CaptureConfig, CaptureConfigError> {
        let rect = window.rect();
        Ok(CaptureConfig {
            target: Capturable::Window(window),
            pixel_format,
            source_rect: Rect {
                origin: Point {
                    x: 0.0,
                    y: 0.0,
                },
                size: rect.size
            },
            output_size: rect.size,
            show_cursor: false,
            impl_capture_config: ImplCaptureConfig::new(rect),
            capture_audio: None,
            buffer_count: 3,
        })
    }

    pub fn with_display(display: CapturableDisplay, pixel_format: CapturePixelFormat) -> CaptureConfig {
        let rect = display.rect();
        CaptureConfig {
            target: Capturable::Display(display),
            pixel_format,
            source_rect: Rect {
                origin: Point {
                    x: 0.0,
                    y: 0.0,
                },
                size: rect.size.scaled(2.0)
            },
            output_size: rect.size,
            show_cursor: false,
            impl_capture_config: ImplCaptureConfig::new(rect),
            capture_audio: None,
            buffer_count: 3,
        }
    }
}

pub struct CaptureStream {
    pub(crate) impl_capture_stream: ImplCaptureStream,
}

impl CaptureStream {
    pub fn test_access(borderless: bool) -> bool {
        ImplCaptureStream::check_access(borderless)
    }

    pub async fn request_access(borderless: bool) -> bool {
        ImplCaptureStream::request_access(borderless).await
    }

    pub fn supported_pixel_formats() -> &'static [CapturePixelFormat] {
        ImplCaptureStream::supported_pixel_formats()
    }

    pub fn new(config: CaptureConfig, callback: impl FnMut(Result<StreamEvent, StreamError>) + Send + 'static) -> Result<Self, StreamCreateError> {
        let boxed_callback = Box::new(callback);
        Ok(Self {
            impl_capture_stream: ImplCaptureStream::new(config, boxed_callback)?
        })
    }

    pub fn stop(&mut self) -> Result<(), StreamStopError> {
        self.impl_capture_stream.stop()
    }
}


