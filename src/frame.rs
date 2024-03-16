use std::{marker::PhantomData, time::{Duration, Instant}, fmt::Debug};

use crate::{platform::platform_impl::{ImplAudioFrame, ImplVideoFrame}, util::*};

/// The rate to capture audio samples
#[derive(Copy, Clone, Debug)]
pub enum AudioSampleRate {
    Hz8000,
    Hz16000,
    Hz24000,
    Hz48000,
}

/// The number of audio channels to capture
#[derive(Copy, Clone, Debug)]
pub enum AudioChannelCount {
    Mono,
    Stereo
}

/// Represents audio channel data in an audio frame
pub enum AudioChannelData<'data> {
    F32(*const f32, usize, PhantomData<&'data ()>),
    I32(*const i32, usize, PhantomData<&'data ()>),
    I16(*const i16, usize, PhantomData<&'data ()>)
}

/// Represents an error getting the data for an audio channel
pub enum AudioBufferError {
    UnsupportedFormat,
    InvalidChannel,
    Other(String)
}

pub(crate) trait AudioCaptureFrame {
    fn sample_rate(&self) -> AudioSampleRate;
    fn channel_count(&self) -> AudioChannelCount;
    fn audio_channel_buffer(&mut self, channel: usize) -> Result<AudioChannelData<'_>, AudioBufferError>;
    fn duration(&self) -> Duration;
    fn origin_time(&self) -> Duration;
    fn capture_time(&self) -> Instant;
    fn frame_id(&self) -> u64;
}

/// A frame of captured audio
pub struct AudioFrame {
    pub(crate) impl_audio_frame: ImplAudioFrame,
}

impl Debug for AudioFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioFrame").finish()
    }
}

impl AudioFrame {
    /// Get the sample rate of the captured audio
    pub fn sample_rate(&self) -> AudioSampleRate {
        self.impl_audio_frame.sample_rate()
    }

    /// Get the channel count of the captured audio
    pub fn channel_count(&self) -> AudioChannelCount {
        self.impl_audio_frame.channel_count()
    }

    /// Get the data buffer for the captured audio channel
    pub fn audio_channel_buffer(&mut self, channel: usize) -> Result<AudioChannelData<'_>, AudioBufferError> {
        self.impl_audio_frame.audio_channel_buffer(channel)
    }

    /// Get the duration of this audio frames
    pub fn duration(&self) -> Duration {
        self.impl_audio_frame.duration()
    }

    /// Get the time since the start of the stream that this audio frame begins at
    pub fn origin_time(&self) -> Duration {
        self.impl_audio_frame.duration()
    }

    /// Get the sequence id of this frame (monotonically increasing)
    /// 
    /// Note: This is separate from video frame ids
    pub fn frame_id(&self) -> u64 {
        self.impl_audio_frame.frame_id()
    }
}

pub(crate) trait VideoCaptureFrame {
    fn logical_frame(&self) -> Rect;
    fn physical_frame(&self) -> Rect;
    fn duration(&self) -> Duration;
    fn origin_time(&self) -> Duration;
    fn capture_time(&self) -> Instant;
    fn frame_id(&self) -> u64;
}

/// Represents a captured video frame
pub struct VideoFrame {
    pub(crate) impl_video_frame: ImplVideoFrame,
}

impl VideoFrame {
    /// Get the sequence id of this video frame (monotonically increasing)
    /// 
    /// Note: This is separate from audio frame ids
    pub fn frame_id(&self) -> u64 {
        self.impl_video_frame.frame_id()
    }

    /// Get the Instant that this frame was delivered to the application
    pub fn capture_time(&self) -> Instant {
        self.impl_video_frame.capture_time()
    }

    /// Get the time since the start of the stream that this frame was generated
    pub fn origin_time(&self) -> Duration {
        self.impl_video_frame.origin_time()
    }

    /// Get the rectangle in system space physical pixels representing the captured area
    pub fn physical_frame(&self) -> Rect {
        self.impl_video_frame.physical_frame()
    }

    /// Get the rectangle in system space logical pixels representing the captured area
    pub fn logical_frame(&self) -> Rect {
        self.impl_video_frame.logical_frame()
    }
}

impl Debug for VideoFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoFrame").finish()
    }
}
