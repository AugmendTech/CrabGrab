#![cfg(target_os = "macos")]
#![cfg(feature = "metal")]

use crate::prelude::VideoFrame;

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
    fn get_texture(&self) -> Result<metal::Texture, MacosVideoFrameError> {
        todo!()
    }
}
