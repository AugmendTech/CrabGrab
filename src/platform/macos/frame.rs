use crate::frame::{VideoCaptureFrame, AudioCaptureFrame};

use super::objc_wrap::CMSampleBuffer;

pub struct MacosVideoFrame {
    pub(crate) sample_buffer: CMSampleBuffer,
    pub(crate) frame_id: u64,
}

impl VideoCaptureFrame for MacosVideoFrame {
    fn logical_frame(&self) -> crate::util::Rect {
        todo!()
    }

    fn physical_frame(&self) -> crate::util::Rect {
        todo!()
    }

    fn content_scale(&self) -> crate::util::Rect {
        todo!()
    }

    fn duration(&self) -> std::time::Duration {
        todo!()
    }

    fn origin_time(&self) -> std::time::Instant {
        todo!()
    }

    fn capture_time(&self) -> std::time::Instant {
        todo!()
    }

    fn frame_id(&self) -> u64 {
        self.frame_id
    }
}

pub struct MacosAudioFrame {
    pub(crate) sample_buffer: CMSampleBuffer,
    pub(crate) frame_id: u64,
}

impl AudioCaptureFrame for MacosAudioFrame {
    fn duration(&self) -> std::time::Duration {
        todo!()
    }

    fn origin_time(&self) -> std::time::Instant {
        todo!()
    }

    fn capture_time(&self) -> std::time::Instant {
        todo!()
    }

    fn frame_id(&self) -> u64 {
        self.frame_id
    }
}
