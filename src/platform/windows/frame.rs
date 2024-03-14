use windows::Graphics::{Capture::Direct3D11CaptureFrame, SizeInt32};

use crate::{prelude::{AudioCaptureFrame, VideoCaptureFrame}, util::{Point, Rect, Size}};

pub struct WindowsVideoFrame {
    pub(crate) frame: Direct3D11CaptureFrame,
    pub(crate) frame_id: u64,
}

impl VideoCaptureFrame for WindowsVideoFrame {
    fn logical_frame(&self) -> Rect {
        let size = self.frame.ContentSize().unwrap_or(SizeInt32::default());
        Rect {
            size: Size {
                width: size.Width as f64,
                height: size.Height as f64,
            },
            origin: Point::ZERO
        }
    }

    fn physical_frame(&self) -> Rect {
        todo!()
    }

    fn duration(&self) -> std::time::Duration {
        todo!()
    }

    fn origin_time(&self) -> std::time::Duration {
        todo!()
    }

    fn capture_time(&self) -> std::time::Instant {
        todo!()
    }

    fn frame_id(&self) -> u64 {
        self.frame_id
    }
}

pub struct WindowsAudioFrame {

}

impl AudioCaptureFrame for WindowsAudioFrame {
    fn sample_rate(&self) -> crate::prelude::AudioSampleRate {
        todo!()
    }

    fn channel_count(&self) -> crate::prelude::AudioChannelCount {
        todo!()
    }

    fn audio_channel_buffer(&mut self, channel: usize) -> Result<crate::prelude::AudioChannelData<'_>, crate::prelude::AudioBufferError> {
        todo!()
    }

    fn duration(&self) -> std::time::Duration {
        todo!()
    }

    fn origin_time(&self) -> std::time::Duration {
        todo!()
    }

    fn capture_time(&self) -> std::time::Instant {
        todo!()
    }

    fn frame_id(&self) -> u64 {
        todo!()
    }
}

