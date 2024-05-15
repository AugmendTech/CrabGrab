use std::sync::Arc;
use std::{error::Error, fmt::Display};

use crate::prelude::{CaptureConfig, CaptureStream, VideoFrame};

#[cfg(target_os = "macos")]
use crate::platform::macos::{capture_stream::MacosCaptureConfig, frame::MacosVideoFrame};
#[cfg(target_os = "macos")]
use crate::feature::metal::*;
use metal::MTLStorageMode;
#[cfg(target_os = "macos")]
use metal::MTLTextureUsage;

#[cfg(target_os = "windows")]
use crate::platform::windows::capture_stream::WindowsCaptureConfig;
#[cfg(target_os = "windows")]
use crate::feature::dx11::*;
#[cfg(target_os = "windows")]
use crate::feature::dxgi::*;
#[cfg(target_os = "windows")]
use windows::{core::{Interface, ComInterface}, Graphics::DirectX::DirectXPixelFormat, Win32::Graphics::{Dxgi::IDXGIDevice, Direct3D11on12::ID3D11On12Device2, Direct3D11::{ID3D11Texture2D, ID3D11Device}, Direct3D::D3D_FEATURE_LEVEL_12_0, Direct3D11::D3D11_CREATE_DEVICE_BGRA_SUPPORT, Direct3D11on12::D3D11On12CreateDevice, Direct3D12::{ID3D12CommandQueue, ID3D12Device, ID3D12Resource, D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET, D3D12_RESOURCE_FLAG_DENY_SHADER_RESOURCE}}};
#[cfg(target_os = "windows")]
use std::ffi::c_void;

/// A capture config which can be supplied with a Wgpu device
pub trait WgpuCaptureConfigExt: Sized {
    fn with_wgpu_device(self, device: Arc<dyn AsRef<wgpu::Device> + Send + Sync + 'static>) -> Result<Self, String>;
}

impl WgpuCaptureConfigExt for CaptureConfig {
    /// Supply a Wgpu device to the config, allowing the generation of Wgpu textures from video frames
    fn with_wgpu_device(self, wgpu_device: Arc<dyn AsRef<wgpu::Device> + Send + Sync + 'static>) -> Result<Self, String> {
        #[cfg(target_os = "macos")]
        {
            unsafe {
                let device = AsRef::<wgpu::Device>::as_ref(&*wgpu_device).as_hal::<wgpu::hal::api::Metal, _, _>(move |device| {
                    if let Some(device) = device {
                        Some(device.raw_device().lock().clone())
                    } else {
                        None
                    }
                }).expect("Expected metal device underneath wgpu");
                Ok(Self {
                    impl_capture_config: MacosCaptureConfig {
                        metal_device: device,
                        wgpu_device: Some(wgpu_device.clone()),
                        ..self.impl_capture_config
                    },
                    ..self
                })
            }
        }
        #[cfg(target_os = "windows")]
        {
            unsafe {
                if let Some(d3d11_device) = 
                    AsRef::<wgpu::Device>::as_ref(&*wgpu_device).as_hal::<wgpu::hal::api::Dx12, _, _>(move |device| {
                        device.map(|device| {
                            device.raw_device().AddRef();
                            let raw_device_ptr = device.raw_device().as_mut_ptr() as *mut c_void;
                            let d3d12_device = ID3D12Device::from_raw_borrowed(&raw_device_ptr).unwrap();
                            device.raw_queue().AddRef();
                            let raw_queue_ptr = device.raw_queue().as_mut_ptr() as *mut c_void;
                            let d3d12_queue = ID3D12CommandQueue::from_raw_borrowed(&raw_queue_ptr).unwrap().to_owned();
                            let d3d12_queue_iunknown = d3d12_queue.cast().unwrap();
                            let mut d3d11_device: Option<ID3D11Device> = None;
                            let mut d3d11_device_context = None;
                            D3D11On12CreateDevice(
                                d3d12_device,
                                D3D11_CREATE_DEVICE_BGRA_SUPPORT.0,
                                Some(&[D3D_FEATURE_LEVEL_12_0]),
                                Some(&[Some(d3d12_queue_iunknown)]),
                                0,
                                Some(&mut d3d11_device as *mut _),
                                Some(&mut d3d11_device_context as *mut _),
                                None
                            ).map_err(|error| format!("Failed to create d3d11 device from wgpu d3d12 device: {}", error.to_string()))?;
                            Result::<_, String>::Ok(d3d11_device.unwrap())
                        })
                    }).flatten() {
                        let d3d11_device = d3d11_device?;
                        let d3d11on12_device: ID3D11On12Device2 = d3d11_device.cast()
                            .map_err(|error| format!("Failed to cast d3d11 device to d3d11on12 device: {}", error.to_string()))?;
                        let dxgi_device: IDXGIDevice = d3d11on12_device.cast()
                            .map_err(|error| format!("Failed to cast d3d11on12 device to dxgi device: {}", error.to_string()))?;
                        let dxgi_adapter = dxgi_device.GetAdapter()
                            .map_err(|error| format!("Failed to get to dxgi adapter: {}", error.to_string()))?;
                        Ok(Self {
                            impl_capture_config: WindowsCaptureConfig {
                                d3d11_device: Some(d3d11_device),
                                wgpu_device: Some(wgpu_device),
                                dxgi_adapter: Some(dxgi_adapter),
                                ..self.impl_capture_config
                            },
                            ..self
                        })
                } else {
                    Err("Unimplemented for wgpu's vulkan backend".into())
                }   
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// Identifies planes of a video frame
pub enum WgpuVideoFramePlaneTexture {
     /// The single RGBA plane for an RGBA format frame
     Rgba,
     /// The Luminance (Y, brightness) plane for a YCbCr format frame
     Luminance,
     /// The Chrominance (CbCr, Blue/Red) plane for a YCbCr format frame
     Chroma
}


/// Represents an error getting the texture from a video frame
#[derive(Clone, Debug)]
pub enum WgpuVideoFrameError {
    /// the backend texture couldn't be fetched
    NoBackendTexture,
    /// The requested plane isn't valid for this frame
    InvalidVideoPlaneTexture,
    /// No Wgpu device was supplied to the capture stream
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

/// A video frame which can be used to create Wgpu textures
pub trait WgpuVideoFrameExt {
    /// Get the texture for the given plane of the video frame
    fn get_wgpu_texture(&self, plane: WgpuVideoFramePlaneTexture, label: Option<&'static str>) -> Result<wgpu::Texture, WgpuVideoFrameError>;
}

impl WgpuVideoFrameExt for VideoFrame {
    fn get_wgpu_texture(&self, plane: WgpuVideoFramePlaneTexture, label: Option<&'static str>) -> Result<wgpu::Texture, WgpuVideoFrameError> {
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
            match MetalVideoFrameExt::get_metal_texture(self, metal_plane) {
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
                                metal::MTLPixelFormat::RGBA8Sint => wgpu::TextureFormat::Rgba8Sint,
                                metal::MTLPixelFormat::RGBA8Uint => wgpu::TextureFormat::Rgba8Uint,
                                metal::MTLPixelFormat::RGBA8Unorm => wgpu::TextureFormat::Rgba8Unorm,
                                metal::MTLPixelFormat::RGBA8Unorm_sRGB => wgpu::TextureFormat::Rgba8UnormSrgb,
                                metal::MTLPixelFormat::RGBA8Snorm => wgpu::TextureFormat::Rgba8Snorm,
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
                                _ => return Err(WgpuVideoFrameError::Other(format!("Unsupported metal texture format: {:?}", metal_texture.pixel_format()))),
                            },
                            usage: {
                                let metal_usage = metal_texture.usage();
                                let storage_mode = metal_texture.storage_mode();
                                if metal_usage.contains(MTLTextureUsage::RenderTarget) { wgpu::TextureUsages::RENDER_ATTACHMENT } else { wgpu::TextureUsages::empty() }.union(
                                    if metal_usage.contains(MTLTextureUsage::ShaderRead ) { wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING } else { wgpu::TextureUsages::empty() } ).union( 
                                    if metal_usage.contains(MTLTextureUsage::ShaderWrite) { wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::STORAGE_BINDING } else { wgpu::TextureUsages::empty() } ).union(
                                        match storage_mode {
                                            MTLStorageMode::Managed |
                                            MTLStorageMode::Private |
                                            MTLStorageMode::Shared => wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
                                            MTLStorageMode::Memoryless => wgpu::TextureUsages::empty(),
                                        }
                                    )
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
                        Ok((&*wgpu_device).as_ref().create_texture_from_hal::<wgpu::hal::api::Metal>(wgpu_metal_texture, &descriptor))
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
            if plane != WgpuVideoFramePlaneTexture::Rgba {
                return Err(WgpuVideoFrameError::InvalidVideoPlaneTexture);
            }
            let wgpu_device = self.impl_video_frame.wgpu_device.as_ref()
                .ok_or(WgpuVideoFrameError::NoWgpuDevice)?.clone();
            let (frame_texture, pixel_format) = WindowsDx11VideoFrame::get_dx11_texture(self)
                .map_err(|_| WgpuVideoFrameError::NoBackendTexture)?;
            let wgpu_format = match pixel_format {
                DirectXPixelFormat::B8G8R8A8Typeless => wgpu::TextureFormat::Bgra8Unorm,
                DirectXPixelFormat::B8G8R8A8UIntNormalized => wgpu::TextureFormat::Bgra8Unorm,
                DirectXPixelFormat::B8G8R8A8UIntNormalizedSrgb => wgpu::TextureFormat::Bgra8UnormSrgb,
                DirectXPixelFormat::R10G10B10A2Typeless => wgpu::TextureFormat::Rgb10a2Uint,
                DirectXPixelFormat::R10G10B10A2UInt => wgpu::TextureFormat::Rgb10a2Uint,
                DirectXPixelFormat::R10G10B10A2UIntNormalized => wgpu::TextureFormat::Rgb10a2Unorm,
                DirectXPixelFormat::R16G16B16A16Float => wgpu::TextureFormat::Rgba16Float,
                _ => return Err(WgpuVideoFrameError::Other("Unsupported DirectXPixelFormat".to_string()))
            };
            unsafe {
                let size = self.size();
                let wgpu_size = wgpu::Extent3d {
                    width: size.width as u32,
                    height: size.height as u32,
                    depth_or_array_layers: 1,
                };
                let d3d11on12_device = self.impl_video_frame.device.cast::<ID3D11On12Device2>().unwrap();
                AsRef::as_ref(&*wgpu_device).as_hal::<wgpu::hal::api::Dx12, _, _>(|wgpu_dx12_device| {
                    let raw_queue_ptr = wgpu_dx12_device.unwrap().raw_queue().as_mut_ptr() as *mut c_void;
                    let d3d12_queue = ID3D12CommandQueue::from_raw_borrowed(&raw_queue_ptr).unwrap().to_owned();
                    let d3d12_texture_resource = d3d11on12_device.UnwrapUnderlyingResource::<&ID3D11Texture2D, &ID3D12CommandQueue, ID3D12Resource>(&frame_texture, &d3d12_queue)
                        .map_err(|error| WgpuVideoFrameError::Other(format!("Failed to unwrap d3d11on12 texture: {}", error.to_string())))?;
                    let d3d12_texture_desc = d3d12_texture_resource.GetDesc();
                    let hal_texture = wgpu::hal::dx12::Device::texture_from_raw(
                        d3d12::ComPtr::from_raw(d3d12_texture_resource.into_raw() as *mut _),
                        wgpu_format,
                        wgpu::TextureDimension::D2,
                        wgpu_size,
                        1,
                        1
                    );
                    let desc = wgpu::TextureDescriptor {
                        label,
                        size: wgpu_size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu_format,
                        usage: {
                            if d3d12_texture_desc.Flags.contains(D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET) { wgpu::TextureUsages::RENDER_ATTACHMENT } else { wgpu::TextureUsages::empty() }.union(
                                if !d3d12_texture_desc.Flags.contains(D3D12_RESOURCE_FLAG_DENY_SHADER_RESOURCE) { wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING } else { wgpu::TextureUsages::empty() }
                            ).union(wgpu::TextureUsages::COPY_SRC)
                        },
                        view_formats: &[]
                    };
                    Ok((*wgpu_device).as_ref().create_texture_from_hal::<wgpu::hal::api::Dx12>(hal_texture, &desc))
                }).unwrap()
            }
        }
    }
}

/// A capture stream which may have had a Wgpu device instance supplied to it
pub trait WgpuCaptureStreamExt {
    /// Gets the Wgpu device wrapper supplied to `CaptureConfig::with_wgpu_device(..)`
    fn get_wgpu_device_wrapper(&self) -> Option<Arc<dyn AsRef<wgpu::Device> + Send + Sync + 'static>>;
    /// Gets the Wgpu device referenced by device wrapper supplied to `CaptureConfig::with_wgpu_device(..)`
    fn get_wgpu_device(&self) -> Option<&wgpu::Device>;
}

impl WgpuCaptureStreamExt for CaptureStream {
    fn get_wgpu_device(&self) -> Option<&wgpu::Device> {
        #[cfg(target_os = "macos")]
        { self.impl_capture_stream.wgpu_device.as_ref().map(|wgpu_device| AsRef::<wgpu::Device>::as_ref(wgpu_device.as_ref())) }
        #[cfg(target_os = "windows")]
        { self.impl_capture_stream.wgpu_device.as_ref().map(|wgpu_device| AsRef::<wgpu::Device>::as_ref(wgpu_device.as_ref())) }
    }

    fn get_wgpu_device_wrapper(&self) -> Option<Arc<dyn AsRef<wgpu::Device> + Send + Sync + 'static>> {
        #[cfg(target_os = "macos")]
        { self.impl_capture_stream.wgpu_device.clone() }
        #[cfg(target_os = "windows")]
        { self.impl_capture_stream.wgpu_device.clone() }
    }
}
