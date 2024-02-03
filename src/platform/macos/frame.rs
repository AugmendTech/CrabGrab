use crate::frame::{VideoCaptureFrame, AudioCaptureFrame};

pub struct MacosVideoFrame {

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
        todo!()
    }
}

#[cfg(feature="metal")]
impl MetalVideoFrame for MacosVideoFrame {
    fn get_texture() -> metal::Texture;
}

pub struct MacosAudioFrame {

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
        todo!()
    }
}
