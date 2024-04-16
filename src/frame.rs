#![allow(unused)]
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
    F32(AudioChannelDataSamples<'data, f32>),
    I32(AudioChannelDataSamples<'data, i32>),
    I16(AudioChannelDataSamples<'data, i16>),
}

// Wraps a "slice" of audio data for one channel, handling interleaving/stride
pub struct AudioChannelDataSamples<'data, T> {
    pub(crate) data: *const u8,
    pub(crate) stride: usize,
    pub(crate) length: usize,
    pub(crate) phantom_lifetime: PhantomData<&'data T>,
}

impl<T: Copy> AudioChannelDataSamples<'_, T> {
    fn get(&self, i: usize) -> T {
        let ptr = self.data.wrapping_add(self.stride * i);
        unsafe { *(ptr as *const T) }
    }

    fn length(&self) -> usize {
        self.length
    }
}

/// Represents an error getting the data for an audio channel
pub enum AudioBufferError {
    // The audio sample format was not supported
    UnsupportedFormat,
    // The requested channel number wasn't present
    InvalidChannel,
    Other(String)
}

pub(crate) trait AudioCaptureFrame {
    fn sample_rate(&self) -> AudioSampleRate;
    fn channel_count(&self) -> AudioChannelCount;
    fn audio_channel_buffer(&mut self, channel: usize) -> Result<AudioChannelData<'_>, AudioBufferError>;
    fn duration(&self) -> Duration;
    fn origin_time(&self) -> Duration;
    fn frame_id(&self) -> u64;
}

/// A frame of captured audio
pub struct AudioFrame {
    pub(crate) impl_audio_frame: ImplAudioFrame,
}

unsafe impl Send for AudioFrame {}
unsafe impl Sync for AudioFrame {}

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
    fn size(&self) -> Size;
    fn dpi(&self) -> f64;
    fn duration(&self) -> Duration;
    fn origin_time(&self) -> Duration;
    fn capture_time(&self) -> Instant;
    fn frame_id(&self) -> u64;
}

/// A frame of captured video
pub struct VideoFrame {
    pub(crate) impl_video_frame: ImplVideoFrame,
}

unsafe impl Send for VideoFrame {}
unsafe impl Sync for VideoFrame {}

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

    /// Get the raw size of the frame
    /// 
    /// For planar image formats, this is the size of the largest plane
    pub fn size(&self) -> Size {
        self.impl_video_frame.size()
    }

    /// Get the dpi of the contents of the frame (accounting for capture scaling)
    pub fn dpi(&self) -> f64 {
        self.impl_video_frame.dpi()
    }
}

impl Debug for VideoFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoFrame").finish()
    }
}
