use std::fmt::Debug;
use std::{error::Error, fmt::Display};

use crate::platform::platform_impl::{ImplAudioCaptureConfig, ImplCaptureAccessToken, ImplCaptureConfig, ImplCaptureStream};
use crate::capturable_content::Capturable;
use crate::prelude::{AudioChannelCount, AudioFrame, AudioSampleRate, CapturableDisplay, CapturableWindow, VideoFrame};
use crate::util::Size;

/// Represents an event in a capture stream
#[derive(Debug)]
pub enum StreamEvent {
    /// This event is produced when the stream receives a new audio packet
    Audio(AudioFrame),
    /// This event is produced when the stream receives a new video frame
    Video(VideoFrame),
    /// This event is produced when the stream goes idle - IE when no new frames are expected for some time, like when a window minimizes
    Idle,
    /// This event is produced once at the end of the stream
    End,
}

/// This represents an error during a stream, for example a failure to retrieve a video or audio frame
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

/// This represents an error when creating a capture stream
#[derive(Debug, Clone)]
pub enum StreamCreateError {
    Other(String),
    /// The supplied pixel format is unsupported by the implementation
    UnsupportedPixelFormat,
    //GpuLost,
    /// Requested features are not authorized
    UnauthorizedFeature(String),
}

unsafe impl Send for StreamCreateError {}
unsafe impl Sync for StreamCreateError {}


impl Display for StreamCreateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(message) => f.write_fmt(format_args!("StreamCreateError::Other(\"{}\")", message)),
            Self::UnsupportedPixelFormat => f.write_fmt(format_args!("StreamCreateError::UnsupportedPixelFormat")),
            Self::UnauthorizedFeature(feature) => f.write_fmt(format_args!("StreamCreateError::UnauthorizedFeature({})", feature)),
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

/// This represents an error while stopping a stream
#[derive(Debug)]
pub enum StreamStopError {
    Other(String),
    /// The stream was already stopped
    AlreadyStopped,
    //GpuLost,
}

unsafe impl Send for StreamStopError {}
unsafe impl Sync for StreamStopError {}

impl Display for StreamStopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(message) => f.write_fmt(format_args!("StreamStopError::Other(\"{}\")", message)),
            Self::AlreadyStopped => f.write_fmt(format_args!("StreamStopError::AlreadyStopped")),
        }
    }
}

impl Error for StreamStopError {
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

/// Configuration settings for audio streams
#[derive(Clone, Debug)]
#[allow(unused)]
pub struct AudioCaptureConfig {
    pub(crate) sample_rate: AudioSampleRate, 
    pub(crate) channel_count: AudioChannelCount,
    pub(crate) impl_capture_audio_config: ImplAudioCaptureConfig,
}

impl AudioCaptureConfig {
    /// Creates a new audio capture config with default settings:
    /// * 24000 Hz
    /// * Mono
    pub fn new() -> Self {
        Self {
            sample_rate: AudioSampleRate::Hz24000,
            channel_count: AudioChannelCount::Mono,
            impl_capture_audio_config: ImplAudioCaptureConfig::new()
        }
    }
}

/// The pixel format of returned video frames
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CapturePixelFormat {
    /// One plane, 4 channels, 8 bits per channel: { b: u8, g: u8, r: u8, a: u8 }, full range: [0, 255]
    Bgra8888,
    /// One plane, 4 channels, 10 bits per color channel, two bits for alpha: { a: u2, r: u10, g: u10, b: u10 }, rgb range: [0, 1023], alpha range: [0, 3]
    Argb2101010,
    /// Two planes:
    /// * 1 channel, luminance (Y), 8 bits per pixel, video range: [16, 240]
    /// * 2 channels, chrominance (CbCr) 8 bits bits per channel per two pixels vertically, range: [0, 255]
    V420,
    /// Two planes:
    /// * 1 channel, luminance (Y), 8 bits per pixel, full range: [0, 255]
    /// * 2 channels, chrominance (CbCr) 8 bits bits per channel per two pixels vertically, range: [0, 255]
    F420,
}

/// Configuration settings for a capture stream
#[derive(Clone, Debug)]
pub struct CaptureConfig {
    pub(crate) target: Capturable,
    pub(crate) output_size: Size,
    pub(crate) show_cursor: bool,
    pub(crate) pixel_format: CapturePixelFormat,
    pub(crate) capture_audio: Option<AudioCaptureConfig>,
    pub(crate) impl_capture_config: ImplCaptureConfig,
    pub(crate) buffer_count: usize,
    pub(crate) excluded_windows: Option<Vec<CapturableWindow>>
}

/// Represents an error creating the capture config
#[derive(Debug, Clone)]
pub enum CaptureConfigError {
    /// The pixel format is unsupported by the implementation
    UnsupportedPixelFormat,
    /// The buffer count is out of the valid range for the implementation
    InvalidBufferCount,
}


unsafe impl Send for CaptureConfigError {}
unsafe impl Sync for CaptureConfigError {}

impl Display for CaptureConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedPixelFormat => f.write_fmt(format_args!("CaptureConfigError::UnsupportedPixelFormat")),
            Self::InvalidBufferCount => f.write_fmt(format_args!("CaptureConfigError::InvalidBufferCount")),
        }
    }
}

impl Error for CaptureConfigError {
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

impl CaptureConfig {
    /// Create a capture configuration for a given capturable window
    pub fn with_window(window: CapturableWindow, pixel_format: CapturePixelFormat) -> Result<CaptureConfig, CaptureConfigError> {
        let rect = window.rect();
        Ok(CaptureConfig {
            target: Capturable::Window(window),
            pixel_format,
            output_size: rect.size,
            show_cursor: false,
            impl_capture_config: ImplCaptureConfig::new(),
            capture_audio: None,
            buffer_count: 3,
            excluded_windows: None
        })
    }

    /// Create a capture configuration for a given capturable display
    pub fn with_display(display: CapturableDisplay, pixel_format: CapturePixelFormat, excluded_windows: Option<Vec<CapturableWindow>>) -> CaptureConfig {
        let rect = display.rect();
        CaptureConfig {
            target: Capturable::Display(display),
            excluded_windows: excluded_windows,
            pixel_format,
            output_size: rect.size,
            show_cursor: false,
            impl_capture_config: ImplCaptureConfig::new(),
            capture_audio: None,
            buffer_count: 3,
        }
    }

    /// Configure the buffer count - the number of frames in the capture queue.
    /// 
    /// Higher numbers mean higher latency, but smoother performance
    pub fn with_buffer_count(self, buffer_count: usize) -> Self {
        Self {
            buffer_count,
            ..self
        }
    }

    /// Configure whether the cursor is visible in the capture
    pub fn with_show_cursor(self, show_cursor: bool) -> Self {
        Self {
            show_cursor,
            ..self
        }
    }

    /// Configure the output texture size - by default, this will match the captured content at the time of enumeration
    pub fn with_output_size(self, output_size: Size) -> Self {
        Self {
            output_size,
            ..self
        }
    }
}

/// Represents an active capture stream
pub struct CaptureStream {
    pub(crate) impl_capture_stream: ImplCaptureStream,
}

unsafe impl Send for CaptureStream {}

/// Represents programmatic capture access
#[derive(Clone, Copy, Debug)]
pub struct CaptureAccessToken {
    pub(crate) impl_capture_access_token: ImplCaptureAccessToken
}

unsafe impl Send for CaptureAccessToken {}
unsafe impl Sync for CaptureAccessToken {}

impl CaptureAccessToken {
    pub fn allows_borderless(&self) -> bool {
        self.impl_capture_access_token.allows_borderless()
    }
}

impl CaptureStream {
    /// Test whether the calling application has permission to capture content
    pub fn test_access(borderless: bool) -> Option<CaptureAccessToken> {
        ImplCaptureStream::check_access(borderless).map(|impl_capture_access_token|
            CaptureAccessToken {
                impl_capture_access_token
            }
        )
    }

    /// Prompt the user for permission to capture content
    pub async fn request_access(borderless: bool) -> Option<CaptureAccessToken> {
        ImplCaptureStream::request_access(borderless).await.map(|impl_capture_access_token|
            CaptureAccessToken {
                impl_capture_access_token
            }
        )
    }

    /// Gets the implementation's supported pixel formats
    pub fn supported_pixel_formats() -> &'static [CapturePixelFormat] {
        ImplCaptureStream::supported_pixel_formats()
    }

    /// Start a new capture stream with the given stream callback
    pub fn new(token: CaptureAccessToken, config: CaptureConfig, callback: impl FnMut(Result<StreamEvent, StreamError>) + Send + 'static) -> Result<Self, StreamCreateError> {
        let boxed_callback = Box::new(callback);
        Ok(Self {
            impl_capture_stream: ImplCaptureStream::new(token.impl_capture_access_token, config, boxed_callback)?
        })
    }

    /// Stop the capture
    pub fn stop(&mut self) -> Result<(), StreamStopError> {
        self.impl_capture_stream.stop()
    }
}


