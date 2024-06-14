use std::{cell::{Ref, RefCell}, marker::PhantomData, sync::Arc, time::{Duration, Instant}};
use crate::{frame::{AudioCaptureFrame, VideoCaptureFrame}, prelude::{AudioBufferError, AudioChannelCount, AudioChannelData, AudioChannelDataSamples, AudioSampleRate, Point}, util::{Rect, Size}};

pub(crate) struct LinuxX11VideoFrame {
    frame_id: u64
}

pub(crate) struct LinuxDummyAudioFrame {
    frame_id: u64
}

impl VideoCaptureFrame for LinuxX11VideoFrame {
    fn size(&self) -> Size {
        todo!()
    }

    fn dpi(&self) -> f64 {
        todo!()
    }

    fn duration(&self) -> Duration {
        todo!()    
    }

    fn capture_time(&self) -> Instant {
        todo!()
    }

    fn frame_id(&self) -> u64 {
        self.frame_id
    }

    fn content_rect(&self) -> Rect {
        todo!()
    }
}

impl AudioCaptureFrame for LinuxDummyAudioFrame {
    fn sample_rate(&self) -> AudioSampleRate {
        todo!()
    }

    fn channel_count(&self) -> AudioChannelCount {
        todo!()
    }

    fn audio_channel_buffer(&mut self, channel: usize) -> Result<AudioChannelData<'_>, AudioBufferError> {
        todo!()
    }

    fn duration(&self) -> Duration {
        todo!()
    }

    fn origin_time(&self) -> Duration {
        todo!()
    }

    fn frame_id(&self) -> u64 {
        self.frame_id
    }
}

