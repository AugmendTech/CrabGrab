use std::{marker::PhantomData, time::{Duration, Instant}, fmt::Debug};

use crate::{platform::platform_impl::{ImplAudioFrame, ImplVideoFrame}, util::*};

#[derive(Copy, Clone, Debug)]
pub enum AudioSampleRate {
    Hz8000,
    Hz16000,
    Hz24000,
    Hz48000,
}

#[derive(Copy, Clone, Debug)]
pub enum AudioChannelCount {
    Mono,
    Stereo
}

pub enum AudioChannelData<'data> {
    F32(*const f32, usize, PhantomData<&'data ()>),
    I32(*const i32, usize, PhantomData<&'data ()>),
    I16(*const i16, usize, PhantomData<&'data ()>)
}

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

pub struct AudioFrame {
    pub(crate) impl_audio_frame: ImplAudioFrame,
}

impl Debug for AudioFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioFrame").finish()
    }
}

impl AudioFrame {
    pub fn sample_rate(&self) -> AudioSampleRate {
        self.impl_audio_frame.sample_rate()
    }

    pub fn channel_count(&self) -> AudioChannelCount {
        self.impl_audio_frame.channel_count()
    }

    pub fn audio_channel_buffer(&mut self, channel: usize) -> Result<AudioChannelData<'_>, AudioBufferError> {
        self.impl_audio_frame.audio_channel_buffer(channel)
    }

    pub fn duration(&self) -> Duration {
        self.impl_audio_frame.duration()
    }

    pub fn origin_time(&self) -> Duration {
        self.impl_audio_frame.duration()
    }

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

pub struct VideoFrame {
    pub(crate) impl_video_frame: ImplVideoFrame,
}

impl VideoFrame {
    pub fn frame_id(&self) -> u64 {
        self.impl_video_frame.frame_id()
    }

    pub fn capture_time(&self) -> Instant {
        self.impl_video_frame.capture_time()
    }

    pub fn origin_time(&self) -> Duration {
        self.impl_video_frame.origin_time()
    }

    pub fn physical_frame(&self) -> Rect {
        self.impl_video_frame.physical_frame()
    }

    pub fn logical_frame(&self) -> Rect {
        self.impl_video_frame.logical_frame()
    }
}

impl Debug for VideoFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("VideoFrame").finish()
    }
}
