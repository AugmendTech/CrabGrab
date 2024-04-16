use std::{error::Error, fmt::Display};

use metal::MTLTextureUsage;

use crate::{platform::{macos::{capture_stream::MacosCaptureConfig, frame::{MacosVideoFrame}}}, prelude::{CaptureConfig, VideoFrame}};

#[cfg(target_os = "macos")]
use crate::feature::metal::*;

#[cfg(target_os = "windows")]
use crate::feature::d3d11::*;
#[cfg(target_os = "windows")]
use crate::feature::dxgi::*;

pub trait WgpuCaptureConfigExt {
    fn with_wgpu_device(self, instance: wgpu::Device) -> Self;
}

impl WgpuCaptureConfigExt for CaptureConfig {
    fn with_wgpu_device(self, device: wgpu::Device) -> Self {
        #[cfg(target_os = "macos")]
        {
            unsafe {
                let device = device.as_hal::<wgpu::hal::api::Metal, _, _>(move |device| {
                    if let Some(device) = device {
                        Some(device.raw_device().lock().clone())
                    } else {
                        None
                    }
                }).expect("Expected metal device underneath wgpu");
                Self {
                    impl_capture_config: MacosCaptureConfig {
                        metal_device: device,
                        ..self.impl_capture_config
                    },
                    ..self
                }
            }
        }
        #[cfg(target_os = "windows")]
        {
            return self;
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
// Identifies planes of a video frame
pub enum WgpuVideoFramePlaneTexture {
     // The single RGBA plane for an RGBA format frame
     Rgba,
     // The Luminance (brightness) plane for a YCbCr format frame
     Luminance,
     // The Chroma (red/blue) plane for a YCbCr format frame
     Chroma
}


/// Represents an error getting the texture from a video frame
#[derive(Clone, Debug)]
pub enum WgpuVideoFrameError {
    /// the backend texture couldn't be fetched
    NoBackendTexture,
    /// The requested plane isn't valid for this frame
    InvalidVideoPlaneTexture,
    /// No wgpu device
    NoWgpuDevice,
    Other(String)
}


impl Display for WgpuVideoFrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoBackendTexture => f.write_str("WgpuVideoFrameError::NoBackendTexture"),
            Self::InvalidVideoPlaneTexture => f.write_str("WgpuVideoFrameError::InvalidVideoPlaneTexture"),
            Self::NoWgpuDevice => f.write_str("WgpuVideoFrameError::NoWgpuDevice"),
            Self::Other(error) => f.write_fmt(format_args!("MacosVideoFrameError::Other(\"{}\")", error)),
        }
    }
}

impl Error for WgpuVideoFrameError {
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

/// A video frame which can be used to create wpgu textures
pub trait WgpuVideoFrameExt {
    /// Get the texture for the given plane of the video frame
    fn get_texture(&self, plane: WgpuVideoFramePlaneTexture, label: Option<&'static str>) -> Result<wgpu::Texture, WgpuVideoFrameError>;
}

impl WgpuVideoFrameExt for VideoFrame {
    fn get_texture(&self, plane: WgpuVideoFramePlaneTexture, label: Option<&'static str>) -> Result<wgpu::Texture, WgpuVideoFrameError> {
        #[cfg(target_os = "macos")]
        {
            let wgpu_device = match &self.impl_video_frame {
                MacosVideoFrame::SCStream(sc_stream_frame) => sc_stream_frame.wgpu_device.clone(),
                MacosVideoFrame::CGDisplayStream(cg_display_stream_frame) => cg_display_stream_frame.wgpu_device.clone(),
            }.ok_or(WgpuVideoFrameError::NoWgpuDevice)?;
            let metal_plane = match plane {
                WgpuVideoFramePlaneTexture::Rgba => MetalVideoFramePlaneTexture::Rgba,
                WgpuVideoFramePlaneTexture::Chroma => MetalVideoFramePlaneTexture::Chroma,
                WgpuVideoFramePlaneTexture::Luminance => MetalVideoFramePlaneTexture::Luminance,
            };
            match MetalVideoFrame::get_texture(self, metal_plane) {
                Ok(metal_texture) => {
                    unsafe {
                        let descriptor = wgpu::TextureDescriptor {
                            label,
                            size: wgpu::Extent3d {
                                width: metal_texture.width() as u32,
                                height: metal_texture.height() as u32,
                                depth_or_array_layers: metal_texture.depth() as u32,
                            },
                            mip_level_count: metal_texture.mipmap_level_count() as u32,
                            sample_count: metal_texture.sample_count() as u32,
                            dimension: match metal_texture.texture_type() {
                                metal::MTLTextureType::D2 |
                                metal::MTLTextureType::D2Multisample=> wgpu::TextureDimension::D2,
                                _ => return Err(WgpuVideoFrameError::Other("Unsupported metal texture type".to_string()))
                            },
                            format: match metal_texture.pixel_format() {
                                metal::MTLPixelFormat::BGRA8Unorm => wgpu::TextureFormat::Bgra8Unorm,
                                metal::MTLPixelFormat::BGRA8Unorm_sRGB => wgpu::TextureFormat::Bgra8UnormSrgb,
                                metal::MTLPixelFormat::RGB10A2Uint => wgpu::TextureFormat::Rgb10a2Uint,
                                metal::MTLPixelFormat::RGB10A2Unorm => wgpu::TextureFormat::Rgb10a2Unorm,
                                metal::MTLPixelFormat::RG8Sint => wgpu::TextureFormat::Rg8Sint,
                                metal::MTLPixelFormat::RG8Snorm => wgpu::TextureFormat::Rg8Snorm,
                                metal::MTLPixelFormat::RG8Uint => wgpu::TextureFormat::Rg8Snorm,
                                metal::MTLPixelFormat::RG8Unorm => wgpu::TextureFormat::Rg8Unorm,
                                metal::MTLPixelFormat::R8Sint => wgpu::TextureFormat::R8Sint,
                                metal::MTLPixelFormat::R8Snorm => wgpu::TextureFormat::R8Snorm,
                                metal::MTLPixelFormat::R8Uint => wgpu::TextureFormat::R8Uint,
                                metal::MTLPixelFormat::R8Unorm => wgpu::TextureFormat::R8Unorm,
                                _ => return Err(WgpuVideoFrameError::Other("Unsupported metal texture format".to_string())),
                            },
                            usage: {
                                let metal_usage = metal_texture.usage();
                                if metal_usage.contains(MTLTextureUsage::RenderTarget) { wgpu::TextureUsages::RENDER_ATTACHMENT } else { wgpu::TextureUsages::empty() }.union(
                                    if metal_usage.contains(MTLTextureUsage::ShaderRead ) { wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING } else { wgpu::TextureUsages::empty() } ).union( 
                                    if metal_usage.contains(MTLTextureUsage::ShaderWrite) { wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING } else { wgpu::TextureUsages::empty() } )
                            },
                            view_formats: &[],
                        };
                        let wgpu_metal_texture = wgpu::hal::metal::Device::texture_from_raw(
                            metal_texture.clone(),
                            descriptor.format,
                            metal_texture.texture_type(),
                            metal_texture.array_length() as u32,
                            metal_texture.mipmap_level_count() as u32,
                            wgpu::hal::CopyExtent { width: metal_texture.width() as u32, height: metal_texture.height() as u32, depth: metal_texture.depth() as u32 }
                        );
                        Ok(wgpu_device.create_texture_from_hal::<wgpu::hal::api::Metal>(wgpu_metal_texture, &descriptor))
                    }
                },
                Err(MacosVideoFrameError::InvalidVideoPlaneTexture) => Err(WgpuVideoFrameError::InvalidVideoPlaneTexture),
                Err(MacosVideoFrameError::NoImageBuffer) |
                Err(MacosVideoFrameError::NoIoSurface) => Err(WgpuVideoFrameError::NoBackendTexture),
                Err(MacosVideoFrameError::Other(e)) => Err(WgpuVideoFrameError::Other(e)),
            }
        }
        #[cfg(target_os = "windows")]
        {

        }
    }
}

