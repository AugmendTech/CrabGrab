use std::sync::Arc;
use std::{error::Error, fmt::Display};

use crate::prelude::{CaptureConfig, CaptureStream, VideoFrame};

#[cfg(target_os = "macos")]
use crate::platform::macos::{capture_stream::MacosCaptureConfig, frame::MacosVideoFrame};
#[cfg(target_os = "macos")]
use crate::feature::metal::*;
#[cfg(target_os = "macos")]
use metal::MTLStorageMode;
#[cfg(target_os = "macos")]
use metal::MTLTextureUsage;
#[cfg(target_os = "windows")]
use d3d12::ComPtr;
#[cfg(target_os = "windows")]
use wgpu::hal::Device;
#[cfg(target_os = "windows")]
use windows::core::PCWSTR;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::WAIT_OBJECT_0;
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{CloseHandle, GENERIC_ALL};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Direct3D11::{D3D11CreateDevice, ID3D11Device5, ID3D11DeviceContext4, ID3D11Fence, D3D11_CREATE_DEVICE_DEBUG, D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Direct3D12::{D3D12_CLEAR_VALUE, D3D12_FENCE_FLAG_NONE, D3D12_FENCE_FLAG_SHARED};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Direct3D12::{ID3D12Fence, D3D12_CPU_PAGE_PROPERTY_UNKNOWN, D3D12_HEAP_FLAG_SHARED, D3D12_HEAP_PROPERTIES, D3D12_HEAP_TYPE_DEFAULT, D3D12_MEMORY_POOL_UNKNOWN, D3D12_RESOURCE_DESC, D3D12_RESOURCE_DIMENSION_TEXTURE2D, D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS, D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, D3D12_RESOURCE_STATE_COMMON, D3D12_TEXTURE_LAYOUT_UNKNOWN};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_TYPELESS;
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory, IDXGIAdapter4, IDXGIFactory5};
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{CreateEventA, WaitForSingleObjectEx, CREATE_EVENT, INFINITE, PROCESS_DELETE, PROCESS_SYNCHRONIZE};
#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{CreateEventExW, THREAD_DELETE, THREAD_SYNCHRONIZE};

#[cfg(target_os = "windows")]
use crate::platform::windows::capture_stream::WindowsCaptureConfig;
#[cfg(target_os = "windows")]
use crate::feature::dx11::*;
#[cfg(target_os = "windows")]
use windows::{core::{Interface, ComInterface}, Graphics::DirectX::DirectXPixelFormat, Win32::Graphics::{Direct3D11::ID3D11Texture2D, Direct3D11::D3D11_CREATE_DEVICE_BGRA_SUPPORT, Direct3D12::{ID3D12CommandQueue, ID3D12Device, ID3D12Resource, D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET, D3D12_RESOURCE_FLAG_DENY_SHADER_RESOURCE}}};
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
                let mut dxgi_adapter_result = Err("Unimplemented for this wgpu backend".to_string());
                AsRef::<wgpu::Device>::as_ref(&*wgpu_device).as_hal::<wgpu::hal::api::Dx12, _, _>(|device| {
                    device.map(|device| {
                        //device.raw_device().AddRef();
                        let raw_device_ptr = device.raw_device().as_mut_ptr() as *mut c_void;
                        let raw_queue_ptr = device.raw_queue().as_mut_ptr() as *mut c_void;
                        let d3d12_device = ID3D12Device::from_raw(raw_device_ptr);
                        let d3d12_queue = ID3D12CommandQueue::from_raw(raw_queue_ptr);
                        let adapter_luid = d3d12_device.GetAdapterLuid();
                        let dxgi_factory: IDXGIFactory5 = match CreateDXGIFactory() {
                            Err(error) => {
                                dxgi_adapter_result = Err(format!("Failed to create dxgi factory: {}", error.to_string()));
                                return;
                            },
                            Ok(factory) => factory,
                        };
                        dxgi_adapter_result = dxgi_factory.EnumAdapterByLuid(adapter_luid)
                            .map_err(|error| format!("Failed to find matching dxgi adapter for wgpu device: {}", error.to_string()))
                            .map(|dxgi_adapter: IDXGIAdapter4| (dxgi_adapter, d3d12_device, d3d12_queue));
                    })
                });
                let (dxgi_adapter, _d3d12_device, _d3d12_queue) = dxgi_adapter_result?;
                let dxgi_adapter = dxgi_adapter.cast::<IDXGIAdapter4>().unwrap();
                let mut d3d11_device = None;
                D3D11CreateDevice (
                    &dxgi_adapter,
                    D3D_DRIVER_TYPE_UNKNOWN,
                    None,
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT | D3D11_CREATE_DEVICE_DEBUG,
                    Some(&[D3D_FEATURE_LEVEL_11_0]),
                    D3D11_SDK_VERSION,
                    Some(&mut d3d11_device),
                    None,
                    None
                ).map_err(|error| format!("Failed to create d3d11 device from dxgi adapter: {}", error.to_string()))?;
                let d3d11_device = d3d11_device.unwrap();
                Ok(Self {
                    impl_capture_config: WindowsCaptureConfig {
                        d3d11_device: Some(d3d11_device),
                        wgpu_device: Some(wgpu_device),
                        dxgi_adapter: Some(dxgi_adapter),
                        ..self.impl_capture_config
                    },
                    ..self
                })
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
            Self::Other(error) => f.write_fmt(format_args!("WgpuVideoFrameError::Other(\"{}\")", error)),
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
            let d3d11_5_device = self.impl_video_frame.device.cast::<ID3D11Device5>()
                .map_err(|error| WgpuVideoFrameError::Other(format!("Device is incompatible with resource sharing interface: {}", error)))?;
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
                AsRef::as_ref(&*wgpu_device).as_hal::<wgpu::hal::api::Dx12, _, _>(|wgpu_dx12_device| {
                    let wgpu_dx12_device = wgpu_dx12_device.unwrap();
                    let d3d12_device_ptr = wgpu_dx12_device.raw_device().as_ptr() as *mut c_void;
                    let d3d12_device = ID3D12Device::from_raw_borrowed(&d3d12_device_ptr).unwrap();
                    let d3d12_queue_ptr = wgpu_dx12_device.raw_queue().as_ptr() as *mut c_void;
                    let d3d12_queue = ID3D12CommandQueue::from_raw_borrowed(&d3d12_queue_ptr).unwrap();

                    let mut frame_desc = D3D11_TEXTURE2D_DESC::default();
                    frame_texture.GetDesc(&mut frame_desc as *mut _);

                    let wgpu_size = wgpu::Extent3d {
                        width: frame_desc.Width,
                        height: frame_desc.Height,
                        depth_or_array_layers: frame_desc.ArraySize,
                    };

                    let d3d12_texture_desc = D3D12_RESOURCE_DESC {
                        Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
                        Alignment: 0,
                        Width: frame_desc.Width as u64,
                        Height: frame_desc.Height,
                        DepthOrArraySize: frame_desc.ArraySize as u16,
                        MipLevels: frame_desc.MipLevels as u16,
                        Format: frame_desc.Format,
                        SampleDesc: frame_desc.SampleDesc,
                        Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
                        Flags: D3D12_RESOURCE_FLAG_ALLOW_RENDER_TARGET | D3D12_RESOURCE_FLAG_ALLOW_SIMULTANEOUS_ACCESS | D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS
                    };
                    let d3d12_texture_heap_properties = D3D12_HEAP_PROPERTIES {
                        Type: D3D12_HEAP_TYPE_DEFAULT,
                        CPUPageProperty: D3D12_CPU_PAGE_PROPERTY_UNKNOWN,
                        MemoryPoolPreference: D3D12_MEMORY_POOL_UNKNOWN,
                        CreationNodeMask: 0,
                        VisibleNodeMask: 0,
                    };  
                    let d3d12_texture_clear_value = D3D12_CLEAR_VALUE {
                        Format: frame_desc.Format,
                        Anonymous: windows::Win32::Graphics::Direct3D12::D3D12_CLEAR_VALUE_0 {
                            Color: [0.0, 0.0, 0.0, 0.0]
                        }
                    };

                    let mut d3d12_texture = None;
                    d3d12_device.CreateCommittedResource(
                        &d3d12_texture_heap_properties as *const _,
                        D3D12_HEAP_FLAG_SHARED,
                        &d3d12_texture_desc as *const _,
                        D3D12_RESOURCE_STATE_COMMON,
                        Some(&d3d12_texture_clear_value),
                        &mut d3d12_texture as *mut _
                    ).map_err(|error| WgpuVideoFrameError::Other(format!("Failed to create d3d12 texture: {}", error.to_string())))?;
                    let d3d12_texture: ID3D12Resource = d3d12_texture.unwrap();

                    let dxgi_shared_texture_handle = d3d12_device.CreateSharedHandle(
                        &d3d12_texture,
                        None,
                        GENERIC_ALL.0,
                        None
                    ).map_err(|error| WgpuVideoFrameError::Other(format!("Failed to share d3d12 texture: {}", error.to_string())))?;

                    let d3d11_shared_texture: ID3D11Texture2D = d3d11_5_device.OpenSharedResource1(dxgi_shared_texture_handle)
                    .map_err(|error| WgpuVideoFrameError::Other(format!("Failed to use dxgi shared texture in d3d11: {}", error.to_string())))?;

                    let d3d12_fence: ID3D12Fence = d3d12_device.CreateFence(0, D3D12_FENCE_FLAG_SHARED)
                        .map_err(|error|  WgpuVideoFrameError::Other(format!("Failed to create fence: {}", error)))?;
                    let fence_event = CreateEventA(None, false, false, None)
                        .map_err(|error|  WgpuVideoFrameError::Other(format!("Failed to create fence event: {}", error)))?;
                    d3d12_fence.SetEventOnCompletion(1, fence_event)
                        .map_err(|error|  WgpuVideoFrameError::Other(format!("Failed to set fence completion event: {}", error.to_string())))?;

                    let dxgi_shared_fence_handle = d3d12_device.CreateSharedHandle(
                        &d3d12_fence,
                        None,
                        GENERIC_ALL.0,
                        None
                    ).map_err(|error| WgpuVideoFrameError::Other(format!("Failed to share fence with dxgi: {}", error.to_string())))?;

                    let mut d3d11_shared_fence = None;
                    d3d11_5_device.OpenSharedFence(dxgi_shared_fence_handle, &mut d3d11_shared_fence)
                        .map_err(|error| WgpuVideoFrameError::Other(format!("Failed to use dxgi shared fence: {}", error.to_string())))?;
                    let d3d11_shared_fence: ID3D11Fence = d3d11_shared_fence.unwrap();

                    {
                        let device_context: ID3D11DeviceContext4 = self.impl_video_frame.device.GetImmediateContext()
                            .map_err(|error| WgpuVideoFrameError::Other(format!("Failed to get d3d11 device context: {}", error.to_string())))?
                            .cast()
                            .map_err(|error| WgpuVideoFrameError::Other(format!("Failed to get d3d11 device context v4: {}", error.to_string())))?;
                        device_context.CopyResource(&d3d11_shared_texture, &frame_texture);
                        device_context.Signal(&d3d11_shared_fence, 1)
                            .map_err(|error| WgpuVideoFrameError::Other(format!("Failed to queue fence signal: {}", error.to_string())))?;
                        drop(frame_texture);
                        drop(d3d11_shared_texture);
                        drop(d3d11_shared_fence);
                        device_context.Flush();
                    }

                    CloseHandle(dxgi_shared_texture_handle)
                        .map_err(|error| WgpuVideoFrameError::Other(format!("Failed to close shared texture handle: {}", error.to_string())))?;

                    let texture_ptr: ComPtr<winapi::um::d3d12::ID3D12Resource> = d3d12::ComPtr::from_raw(d3d12_texture.into_raw() as *mut _);

                    let hal_texture = wgpu::hal::dx12::Device::texture_from_raw(
                        texture_ptr.clone(),
                        wgpu_format,
                        wgpu::TextureDimension::D2,
                        wgpu_size,
                        frame_desc.MipLevels.max(1),
                        frame_desc.SampleDesc.Count
                    );

                    d3d12_queue.Wait(&d3d12_fence, 1)
                        .map_err(|error| WgpuVideoFrameError::Other(format!("Failed to enqueue wait on fence: {}", error.to_string())))?;

                    if WaitForSingleObjectEx(fence_event, INFINITE, false) != WAIT_OBJECT_0 {
                        Err(WgpuVideoFrameError::Other(format!("Failed wait on completion fence")))?
                    }

                    CloseHandle(dxgi_shared_fence_handle)
                        .map_err(|error| WgpuVideoFrameError::Other(format!("Failed to close shared fence handle: {}", error.to_string())))?;
                    CloseHandle(fence_event)
                        .map_err(|error| WgpuVideoFrameError::Other(format!("Failed to close fence event handle: {}", error.to_string())))?;
                    
                    let desc = wgpu::TextureDescriptor {
                        label,
                        size: wgpu_size,
                        mip_level_count: frame_desc.MipLevels.max(1),
                        sample_count: frame_desc.SampleDesc.Count,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu_format,
                        usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING,
                        view_formats: &[wgpu_format]
                    };
                    let result = Ok((*wgpu_device).as_ref().create_texture_from_hal::<wgpu::hal::api::Dx12>(hal_texture, &desc));

                    // dirty hack to reduce the refount
                    std::mem::drop(std::mem::transmute_copy::<_, ComPtr<winapi::um::d3d12::ID3D12Resource>>(&texture_ptr));

                    result
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
