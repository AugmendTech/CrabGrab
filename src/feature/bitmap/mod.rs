#![cfg(feature = "bitmap")]

use half::f16;
use windows::Win32::Graphics::Direct3D11::{D3D11_STANDARD_MULTISAMPLE_QUALITY_LEVELS, D3D11_USAGE_DYNAMIC};

use crate::prelude::VideoFrame;
#[cfg(target_os = "macos")]
use crate::platform::macos::frame::MacosVideoFrame;
#[cfg(target_os = "macos")]
use crate::platform::platform_impl::objc_wrap::CVPixelFormat;

#[cfg(target_os = "windows")]
use crate::feature::dx11::{WindowsDx11VideoFrame, WindowsDx11VideoFrameError};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;
#[cfg(target_os = "windows")]
use windows::Graphics::DirectX::DirectXPixelFormat;
#[cfg(target_os = "windows")]
use windows::core::ComInterface;
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Direct3D11::{D3D11_BOX, D3D11_CPU_ACCESS_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_R10G10B10A2_UNORM, DXGI_SAMPLE_DESC};
#[cfg(target_os = "windows")]
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;

pub struct FrameBitmapBgraUnorm8x4 {
    pub data: Box<[[u8; 4]]>,
    pub width:  usize,
    pub height: usize,
}

pub struct FrameBitmapRgbaUnormPacked1010102 {
    pub data: Box<[u32]>,
    pub width:  usize,
    pub height: usize,
}

pub struct FrameBitmapRgbaF16x4 {
    pub data: Box<[[f16; 4]]>,
    pub width:  usize,
    pub height: usize,
}

pub enum VideoRange {
    Video,
    Full,
}

pub struct FrameBitmapYCbCr {
    pub luma_data: Box<[u8]>,
    pub chroma_data: Box<[[u8; 2]]>,
    pub width: usize,
    pub luma_height: usize,
    pub chroma_height: usize,
    pub range: VideoRange,
}

pub enum FrameBitmap {
    BgraUnorm8x4(FrameBitmapBgraUnorm8x4),
    RgbaUnormPacked1010102(FrameBitmapRgbaUnormPacked1010102),
    RgbaF16x4(FrameBitmapRgbaF16x4),
    YCbCr(FrameBitmapYCbCr),
}

pub trait VideoFrameBitmap {
    fn get_bitmap(&self) -> Result<FrameBitmap, VideoFrameBitmapError>;
}

#[derive(Clone, Debug)]
pub enum VideoFrameBitmapError {
    Other(String),
}

impl VideoFrameBitmap for VideoFrame {
    fn get_bitmap(&self) -> Result<FrameBitmap, VideoFrameBitmapError> {
        #[cfg(target_os = "windows")]
        {
            let (width, height) = self.impl_video_frame.frame_size;
            match self.get_dx11_surface() {
                Err(WindowsDx11VideoFrameError::Other(x)) => Err(VideoFrameBitmapError::Other(x)),
                Ok((surface, pixel_format)) => {
                    let (pixel_size, dxgi_format) = match pixel_format {
                        DirectXPixelFormat::B8G8R8A8UIntNormalized => (4, DXGI_FORMAT_B8G8R8A8_UNORM),
                        DirectXPixelFormat::R10G10B10A2UIntNormalized => (4, DXGI_FORMAT_R10G10B10A2_UNORM),
                        _ => return Err(VideoFrameBitmapError::Other("Unknown or unsupported pixel format on DXGISurface".to_string())),
                    };
                    
                    unsafe {
                        let surface_desc = surface.Description()
                            .map_err(|_| VideoFrameBitmapError::Other("Couldn't get description of frame surface".to_string()))?;
                        let mut new_texture_desc = D3D11_TEXTURE2D_DESC::default();
                        new_texture_desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
                        new_texture_desc.ArraySize = 1;
                        new_texture_desc.BindFlags = 0;
                        new_texture_desc.Width = surface_desc.Width as u32;
                        new_texture_desc.Height = surface_desc.Height as u32;
                        new_texture_desc.MipLevels = 1;
                        new_texture_desc.SampleDesc.Count = 1;
                        new_texture_desc.SampleDesc.Quality = 0;
                        new_texture_desc.Usage.0 = D3D11_USAGE_STAGING.0 | D3D11_USAGE_DYNAMIC.0;
                        new_texture_desc.Format = dxgi_format;
                        let mut staging_texture = Option::<ID3D11Texture2D>::None;
                        let staging_tex_result = self.impl_video_frame.device.CreateTexture2D(&new_texture_desc as *const _, None, Some(&mut staging_texture as *mut _));
                        staging_tex_result.map_err(|error| VideoFrameBitmapError::Other(format!("Failed to create texture: {}", error.to_string())))?;
                        let dxgi_interfce_access: IDirect3DDxgiInterfaceAccess = surface.cast()
                            .map_err(|_| VideoFrameBitmapError::Other("Couldn't create surface interface access".to_string()))?;
                        let surface_texture: ID3D11Texture2D = dxgi_interfce_access.GetInterface()
                            .map_err(|_| VideoFrameBitmapError::Other("Couldn't create surface texture from surface IDirect3DDxgiInterfaceAccess".to_string()))?;
                        let device = self.impl_video_frame.device.GetImmediateContext()
                            .map_err(|_| VideoFrameBitmapError::Other("Couldn't get immediate d3d11 context".to_string()))?;
                        let staging_texture = staging_texture.unwrap();
                        device.CopyResource(&staging_texture, &surface_texture);
                        let mut mapped_resource = D3D11_MAPPED_SUBRESOURCE::default();
                        let map_result = device.Map(&staging_texture, 0, D3D11_MAP_READ, 0, Some(&mut mapped_resource as *mut _));
                        map_result.map_err(|_| VideoFrameBitmapError::Other("Couldn't map staging texture".to_string()))?;
                        match pixel_format {
                            DirectXPixelFormat::B8G8R8A8UIntNormalized => {
                                let mut image_data = vec![[0u8; 4]; width * height];
                                let bpr = mapped_resource.RowPitch as usize;
                                let surface_slice = std::slice::from_raw_parts(mapped_resource.pData as *const u8, bpr * height);
                                for y in 0..height {
                                    let source_slice = bytemuck::cast_slice::<_, [u8; 4]>(&surface_slice[(bpr * y)..(bpr * y + 4 * width)]);
                                    image_data[(width * y)..(width * y + width)].copy_from_slice(source_slice);
                                }
                                let _ = device.Unmap(&staging_texture, 0);
                                Ok(FrameBitmap::BgraUnorm8x4(FrameBitmapBgraUnorm8x4 {
                                    data: image_data.into_boxed_slice(),
                                    width,
                                    height,
                                }))
                            },
                            DirectXPixelFormat::R10G10B10A2UIntNormalized => {
                                let mut image_data = vec![0u32; width * height];
                                let bpr = mapped_resource.RowPitch as usize;
                                let surface_slice = std::slice::from_raw_parts(mapped_resource.pData as *const u8, bpr * height);
                                for y in 0..height {
                                    let source_slice = bytemuck::cast_slice::<_, u32>(&surface_slice[(bpr * y)..(bpr * y + 4 * width)]);
                                    image_data[(width * y)..(width * y + width)].copy_from_slice(source_slice);
                                }
                                let _ = device.Unmap(&staging_texture, 0);
                                Ok(FrameBitmap::RgbaUnormPacked1010102(FrameBitmapRgbaUnormPacked1010102 {
                                    data: image_data.into_boxed_slice(),
                                    width,
                                    height,
                                }))
                            },
                            _ => {
                                Err(VideoFrameBitmapError::Other("Unknown or unsupported pixel format on DXGISurface".to_string()))
                            }
                        }
                    }
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            let iosurface = match &self.impl_video_frame {
                MacosVideoFrame::SCStream(sc_frame) => {
                    match sc_frame.sample_buffer.get_image_buffer().map(|image_buffer| image_buffer.get_iosurface()).flatten() {
                        Some(iosurface) => iosurface,
                        None => return Err(VideoFrameBitmapError::Other("Failed to get iosurface".to_string())),
                    }
                },
                MacosVideoFrame::CGDisplayStream(cg_display_frame) => {
                    cg_display_frame.io_surface.clone()
                }
            };
            if let Ok(lock_gaurd) = iosurface.lock(true, false) {
                let pixel_format = iosurface.get_pixel_format();
                match pixel_format {
                    Some(CVPixelFormat::BGRA8888) => {
                        let bpr = iosurface.get_bytes_per_row();
                        let height = iosurface.get_height();
                        let width = iosurface.get_width();
                        let mut image_data = vec![[0; 4]; width * height];
                        let base_address = lock_gaurd.get_base_address().ok_or(VideoFrameBitmapError::Other("Failed to get base address of iosurface".into()))?;
                        let iosurface_slice = unsafe { std::slice::from_raw_parts(base_address as *const u8, bpr * height) };
                        for y in 0..height {
                            let source_slice = bytemuck::cast_slice::<_, [u8; 4]>(&iosurface_slice[(bpr * y)..(bpr * y + 4 * width)]);
                            image_data[(width * y)..(width * y + width)].copy_from_slice(source_slice);
                        }
                        Ok(FrameBitmap::BgraUnorm8x4(FrameBitmapBgraUnorm8x4 {
                            data: image_data.into_boxed_slice(),
                            width,
                            height,
                        }))
                    },
                    Some(CVPixelFormat::V420) |
                    Some(CVPixelFormat::F420) => {
                        let width = iosurface.get_width();

                        let luma_bpr = iosurface.get_bytes_per_row_of_plane(0);
                        let luma_height = iosurface.get_height_of_plane(0);
                        let mut luma_image_data = vec![0u8; width * luma_height];
                        let luma_base_address = lock_gaurd.get_base_address_of_plane(0).ok_or(VideoFrameBitmapError::Other("Failed to get base address of iosurface".into()))?;
                        let luma_iosurface_slice = unsafe { std::slice::from_raw_parts(luma_base_address as *const u8, luma_bpr * luma_height) };

                        for y in 0..luma_height {
                            let luma_source_slice = &luma_iosurface_slice[(luma_bpr * y)..(luma_bpr * y * width)];
                            luma_image_data[(width * y)..(width * y + width)].copy_from_slice(luma_source_slice);                            
                        }

                        let chroma_bpr = iosurface.get_bytes_per_row_of_plane(1);
                        let chroma_height = iosurface.get_height_of_plane(1);
                        let mut chroma_image_data = vec![[0u8; 2]; width * chroma_height];
                        let chroma_base_address = lock_gaurd.get_base_address_of_plane(1).ok_or(VideoFrameBitmapError::Other("Failed to get base address of iosurface".into()))?;
                        let chroma_iosurface_slice = unsafe { std::slice::from_raw_parts(chroma_base_address as *const u8, chroma_bpr * chroma_height) };

                        for y in 0..chroma_height {
                            let chroma_source_slice = bytemuck::cast_slice::<_, [u8; 2]>(&chroma_iosurface_slice[(chroma_bpr * y)..(chroma_bpr * y + 2 * width)]);
                            chroma_image_data[(width * y)..(width * y + width)].copy_from_slice(chroma_source_slice);
                        }

                        Ok(FrameBitmap::YCbCr(FrameBitmapYCbCr {
                            luma_data: luma_image_data.into_boxed_slice(),
                            chroma_data: chroma_image_data.into_boxed_slice(),
                            width,
                            luma_height,
                            chroma_height,
                            range: if pixel_format == Some(CVPixelFormat::F420) { VideoRange::Full } else { VideoRange::Video }
                        }))
                    },
                    _ => Err(VideoFrameBitmapError::Other("Unknown pixel format on iosurface".to_string()))
                }
            } else {
                Err(VideoFrameBitmapError::Other("Failed to lock iosurface".to_string()))
            }
        }
    }
}


