#![cfg(target_os = "macos")]
#![cfg(feature = "metal")]

use crate::prelude::{CaptureStream, VideoFrame};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MetalVideoFramePlaneTexture {
    Rgba,
    Luminance,
    Chroma
}

/// Represents an error getting the texture from a video frame
#[derive(Clone, Debug)]
pub enum MacosVideoFrameError {
    InvalidVideoPlaneTexture,
    Other(String)
}

pub trait MetalVideoFrame {
    /// Get the texture for the given plane of the video frame
    fn get_texture(&self, plane: MetalVideoFramePlaneTexture) -> Result<metal::Texture, MacosVideoFrameError>;
}

#[cfg(feature="metal")]
impl MetalVideoFrame for VideoFrame {
    fn get_texture(&self, plane: MetalVideoFramePlaneTexture) -> Result<metal::Texture, MacosVideoFrameError> {
        todo!()
    }
}

pub trait MetalCaptureStream {
    fn get_metal_device(&self) -> metal::Device;
}

impl MetalCaptureStream for CaptureStream {
    /// Get the metal device used for the capture stream
    fn get_metal_device(&self) -> metal::Device {
        self.impl_capture_stream.metal_device.clone()
    }
}
