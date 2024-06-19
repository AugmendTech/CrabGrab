use std::{error::Error, fmt::Display};

pub trait AshContext: Send + Sync {
    fn device(&self) -> &ash::Device;
    fn copy_queue(&self) -> &ash::vk::Queue;
    fn texture_allocator(&self) -> &ash::vk::AllocationCallbacks;
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum AshVideoFramePlaneTexture {
    /// The single RGBA plane for an RGBA format frame
    Rgba,
    /// The Luminance (Y, brightness) plane for a YCbCr format frame
    Luminance,
    /// The Chrominance (CbCr, Blue/Red) plane for a YCbCr format frame
    Chroma
}

#[derive(Clone, Debug)]
pub enum AshVideoFrameError {
    Other(String)
}


impl Display for AshVideoFrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(error) => f.write_fmt(format_args!("AshVideoFrameError::Other(\"{}\")", error)),
        }
    }
}

impl Error for AshVideoFrameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }

    fn description(&self) -> &str {
        "description() is deprecated; use Display"
    }

    fn cause(&self) -> Option<&dyn Error> {
        self.source()
    }
}

pub struct AshVideoFrameTexture {
    pub texture: ash::vk::Image,
    pub format: ash::vk::Format,
    pub usage_flags: ash::vk::ImageUsageFlags,
    pub width: usize,
    pub height: usize,
    pub layout: ash::vk::ImageLayout,
}

pub trait AshVideoFrameExt {
    fn get_ash_texture(&self, plane: AshVideoFramePlaneTexture) -> Result<AshVideoFrameTexture, AshVideoFrameError>;
}

pub trait AshCaptureStreamExt {
    // device context functions
}
