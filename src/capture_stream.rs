use std::{error::Error, fmt::Display};

use crate::platform::platform_impl::{ImplCaptureStream, ImplAudioFrame as AudioFrame, ImplVideoFrame as VideoFrame};

pub enum StreamEvent {
    Audio(AudioFrame),
    Video(VideoFrame),
    Stopped,
    StoppedWithError(StreamError)
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
    capture_stream: ImplCaptureStream,
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

impl CaptureStream {
    pub fn new(callback: impl FnMut(StreamEvent)) -> Result<Self, StreamCreateError> {
        Err(StreamCreateError::Other("Not Implemented".into()))
    }
}


