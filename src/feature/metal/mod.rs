#![cfg(target_os = "macos")]
#![cfg(feature = "metal")]

use metal::foreign_types::ForeignType;
use objc::msg_send;
use objc::runtime::Object;
use objc::sel;
use objc::sel_impl;

use crate::platform::platform_impl::objc_wrap::CVPixelFormat;
use crate::prelude::{CaptureStream, VideoFrame};

use std::error::Error;
use std::fmt::Display;

use crate::platform::macos::frame::MacosVideoFrame;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// Identifies planes of a video frame
pub enum MetalVideoFramePlaneTexture {
    /// The single RGBA plane for an RGBA format frame
    Rgba,
    /// The Luminance (brightness) plane for a YCbCr format frame
    Luminance,
    /// The Chroma (red/blue) plane for a YCbCr format frame
    Chroma
}

/// Represents an error getting the texture from a video frame
#[derive(Clone, Debug)]
pub enum MacosVideoFrameError {
    // Could not retreive the IOSurface for this frame
    NoIoSurface,
    // Could not retreive the image buffer for this frame
    NoImageBuffer,
    // The requested plane isn't valid for this frame
    InvalidVideoPlaneTexture,
    Other(String)
}


impl Display for MacosVideoFrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoIoSurface => f.write_str("MacosVideoFrameError::NoIoSurface"),
            Self::NoImageBuffer => f.write_str("MacosVideoFrameError::NoImageBuffer"),
            Self::InvalidVideoPlaneTexture => f.write_str("MacosVideoFrameError::InvalidVideoPlaneTexture"),
            Self::Other(error) => f.write_fmt(format_args!("MacosVideoFrameError::Other(\"{}\")", error)),
        }
    }
}

impl Error for MacosVideoFrameError {
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

/// A video frame which can be used to create metal textures
pub trait MetalVideoFrameExt {
    /// Get the texture for the given plane of the video frame
    fn get_texture(&self, plane: MetalVideoFramePlaneTexture) -> Result<metal::Texture, MacosVideoFrameError>;
}

#[cfg(feature="metal")]
impl MetalVideoFrameExt for VideoFrame {
    fn get_texture(&self, plane: MetalVideoFramePlaneTexture) -> Result<metal::Texture, MacosVideoFrameError> {
        let iosurface_and_metal_device = match &self.impl_video_frame {
            MacosVideoFrame::SCStream(frame) => {
                match frame.sample_buffer.get_image_buffer() {
                    Some(image_buffer) => {
                        match image_buffer.get_iosurface() {
                            Some(iosurface) => {
                                Ok((iosurface, frame.metal_device.clone()))
                            },
                            None => Err(MacosVideoFrameError::NoIoSurface)
                        }
                    },
                    None => Err(MacosVideoFrameError::NoImageBuffer)
                }
            },
            MacosVideoFrame::CGDisplayStream(frame) => {
                Ok((frame.io_surface.clone(), Some(frame.metal_device.clone())))
            }
        }?;
        let (iosurface, metal_device) = iosurface_and_metal_device;
        let pixel_format = match iosurface.get_pixel_format() {
            None => return Err(MacosVideoFrameError::Other("Unable to get pixel format from iosurface".to_string())),
            Some(format) => format
        };
        match pixel_format {
            CVPixelFormat::BGRA8888 => {
                match plane {
                    MetalVideoFramePlaneTexture::Rgba => {},
                    _ => return Err(MacosVideoFrameError::InvalidVideoPlaneTexture),
                }
                unsafe {
                    let device_ref = metal_device.as_ref().unwrap().as_ref();
                    let texture_descriptor = metal::TextureDescriptor::new();
                    texture_descriptor.set_texture_type(metal::MTLTextureType::D2);
                    texture_descriptor.set_pixel_format(metal::MTLPixelFormat::BGRA8Unorm);
                    texture_descriptor.set_width(iosurface.get_width() as u64);
                    texture_descriptor.set_height(iosurface.get_height() as u64);
                    texture_descriptor.set_sample_count(1);
                    texture_descriptor.set_mipmap_level_count(1);
                    texture_descriptor.set_storage_mode(metal::MTLStorageMode::Shared);
                    let texture_ptr: *mut Object = msg_send![device_ref, newTextureWithDescriptor: texture_descriptor.as_ptr() iosurface: iosurface.0 plane: 0];
                    if texture_ptr.is_null() {
                        Err(MacosVideoFrameError::Other("Failed to create metal texture".to_string()))
                    } else {
                        Ok((metal::Texture::from_ptr(texture_ptr as *mut metal::MTLTexture)).to_owned())
                    }
                }
            },
            CVPixelFormat::V420 | CVPixelFormat::F420 => {
                let (plane, texture_index) = match plane {
                    MetalVideoFramePlaneTexture::Luminance => (0, metal::MTLPixelFormat::R8Uint),
                    MetalVideoFramePlaneTexture::Chroma => (1, metal::MTLPixelFormat::RG8Uint),
                    _ => return Err(MacosVideoFrameError::InvalidVideoPlaneTexture),
                };
                unsafe {
                    let device_ref = metal_device.as_ref().unwrap().as_ref();
                    let texture_descriptor = metal::TextureDescriptor::new();
                    texture_descriptor.set_texture_type(metal::MTLTextureType::D2);
                    texture_descriptor.set_pixel_format(texture_index);
                    texture_descriptor.set_width(iosurface.get_width() as u64);
                    texture_descriptor.set_height(iosurface.get_height_of_plane(plane) as u64);
                    texture_descriptor.set_sample_count(1);
                    texture_descriptor.set_mipmap_level_count(1);
                    texture_descriptor.set_storage_mode(metal::MTLStorageMode::Shared);
                    let texture_ptr: *mut Object = msg_send![device_ref, newTextureWithDescriptor: texture_descriptor.as_ptr() iosurface: iosurface.0 plane: plane];
                    if texture_ptr.is_null() {
                        Err(MacosVideoFrameError::Other("Failed to create metal texture".to_string()))
                    } else {
                        Ok((metal::Texture::from_ptr(texture_ptr as *mut metal::MTLTexture)).to_owned())
                    }
                }
            },
            _ => Err(MacosVideoFrameError::Other("Unknown pixel format on iosurface".to_string())),
        }
    }
}

/// A capture stream which interoperates with Metal
pub trait MetalCaptureStreamExt {
    /// Get the metal device used for frame capture
    fn get_metal_device(&self) -> metal::Device;
}

impl MetalCaptureStreamExt for CaptureStream {
    fn get_metal_device(&self) -> metal::Device {
        self.impl_capture_stream.metal_device.clone()
    }
}
