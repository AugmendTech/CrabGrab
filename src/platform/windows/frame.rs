use crate::prelude::{AudioCaptureFrame, VideoCaptureFrame};

pub struct WindowsVideoFrame {

}

impl VideoCaptureFrame for WindowsVideoFrame {
    fn logical_frame(&self) -> crate::util::Rect {
        todo!()
    }

    fn physical_frame(&self) -> crate::util::Rect {
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

