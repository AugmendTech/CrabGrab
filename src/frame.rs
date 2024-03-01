use std::{marker::PhantomData, time::{Duration, Instant}};

use crate::util::*;

pub trait VideoCaptureFrame {
    fn logical_frame(&self) -> Rect;
    fn physical_frame(&self) -> Rect;
    fn duration(&self) -> Duration;
    fn origin_time(&self) -> Duration;
    fn capture_time(&self) -> Instant;
    fn frame_id(&self) -> u64;
}



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

pub trait AudioCaptureFrame {
    fn sample_rate(&self) -> AudioSampleRate;
    fn channel_count(&self) -> AudioChannelCount;
    fn audio_channel_buffer(&mut self, channel: usize) -> Result<AudioChannelData<'_>, AudioBufferError>;
    fn duration(&self) -> Duration;
    fn origin_time(&self) -> Duration;
    fn capture_time(&self) -> Instant;
    fn frame_id(&self) -> u64;
}
