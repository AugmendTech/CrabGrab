use crate::frame::{VideoCaptureFrame, AudioCaptureFrame};

pub struct MacosVideoFrame {

}

impl VideoCaptureFrame for MacosVideoFrame {

}

#[cfg(feature="metal")]
impl MetalVideoFrame for MacosVideoFrame {
    fn get_texture() -> metal::Texture;
}

pub struct MacosAudioFrame {

}

impl AudioCaptureFrame for MacosAudioFrame {
    
}
