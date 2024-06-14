#![cfg(feature = "bitmap")]

use std::error::Error;
use std::fmt::Display;
use std::os::raw::c_void;
use bytemuck::Pod;
use bytemuck::Zeroable;
use parking_lot::Mutex;
use parking_lot::Condvar;
use std::sync::Arc;

use half::f16;

use crate::prelude::CapturePixelFormat;
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
use windows::Win32::Graphics::Direct3D11::{D3D11_CPU_ACCESS_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING};
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_R10G10B10A2_UNORM};
#[cfg(target_os = "windows")]
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Direct3D11::D3D11_USAGE_DYNAMIC;

pub trait ZeroValue {
    fn zero_value() -> Self;
}

impl ZeroValue for u8 {
    fn zero_value() -> Self {
        0
    }
}

impl ZeroValue for [u8; 2] {
    fn zero_value() -> Self {
        [0, 0]
    }
}

impl ZeroValue for [u8; 4] {
    fn zero_value() -> Self {
        [0, 0, 0, 0]
    }
}

impl ZeroValue for [f16; 4] {
    fn zero_value() -> Self {
        [f16::ZERO, f16::ZERO, f16::ZERO, f16::ZERO]
    }
}

impl ZeroValue for u32 {
    fn zero_value() -> Self {
        0
    }
}

#[derive(Clone)]
struct BitmapPool<T: Sized + ZeroValue + Copy> {
    free_bitmaps_and_count: Arc<Mutex<(Vec<Box<[T]>>, usize)>>,
    free_condition: Arc<Condvar>,
    max: usize,
}

impl<T: Sized + ZeroValue + Copy> BitmapPool<T> {
    pub fn new(initial_count: usize, max: usize, initial_resolution: (usize, usize)) -> Arc<Self> {
        let mut free_bitmaps = Vec::new();
        for _ in 0..initial_count {
            free_bitmaps.push(
                vec![T::zero_value(); initial_resolution.0 * initial_resolution.1].into_boxed_slice()
            )
        }
        Arc::new(Self {
            free_bitmaps_and_count: Arc::new(Mutex::new((free_bitmaps, initial_count))),
            free_condition: Arc::new(Condvar::new()),
            max,
        })
    }

    fn make_new_bitmap(self: &Arc<Self>, resolution: (usize, usize)) -> Option<PooledBitmap<T>> {
        Some(PooledBitmap {
            data: PooledBitmapData {
                data: Some(vec![T::zero_value(); resolution.0 * resolution.1].into_boxed_slice()),
                pool: self.clone()
            },
            width: resolution.0,
            height: resolution.1
        })
    }

    pub fn try_get_bitmap(self: &Arc<Self>, resolution: (usize, usize)) -> Option<PooledBitmap<T>> {
        let mut free_bitmaps_and_count = self.free_bitmaps_and_count.lock();
        self.try_get_bitmap_internal(resolution, &mut free_bitmaps_and_count)
    }

    pub fn get_bitmap(self: &Arc<Self>, resolution: (usize, usize)) -> PooledBitmap<T> {
        let mut free_bitmaps_and_count = self.free_bitmaps_and_count.lock();
        loop {
            if let Some(pooled_bitmap) = self.try_get_bitmap_internal(resolution, &mut free_bitmaps_and_count) {
                return pooled_bitmap;
            } else {
                self.free_condition.wait(&mut free_bitmaps_and_count);
            }
        }
    }

    fn try_get_bitmap_internal(self: &Arc<Self>, resolution: (usize, usize), free_bitmaps_and_count: &mut (Vec<Box<[T]>>, usize)) -> Option<PooledBitmap<T>> {
        if let Some(bitmap_data) = free_bitmaps_and_count.0.pop() {
            if bitmap_data.len() <= resolution.0 * resolution.1 {
                return Some(
                    PooledBitmap {
                        data: PooledBitmapData {
                            data: Some(bitmap_data),
                            pool: self.clone()
                        },
                        width: resolution.0,
                        height: resolution.1
                    }
                );
            }
            free_bitmaps_and_count.1 -= 1;
        }
        if free_bitmaps_and_count.1 < self.max {
            return self.make_new_bitmap(resolution);
        }
        None
    }

    pub fn free_pooled(&self) {
        let mut free_bitmaps_and_count = self.free_bitmaps_and_count.lock();
        let count = free_bitmaps_and_count.0.len();
        free_bitmaps_and_count.0.clear();
        free_bitmaps_and_count.1 -= count;
    }
}

struct PooledBitmapData<T: Sized + ZeroValue + Copy> {
    pub data: Option<Box<[T]>>,
    pub pool: Arc<BitmapPool<T>>,
}

impl<T: Sized + ZeroValue + Copy> Drop for PooledBitmapData<T> {
    fn drop(&mut self) {
        if let Some(data) = self.data.take() {
            let mut free_bitmaps_and_count = self.pool.free_bitmaps_and_count.lock();
            free_bitmaps_and_count.0.push(data);
            self.pool.free_condition.notify_all();
        }
    }
}

pub struct PooledBitmap<T: Sized + Copy + ZeroValue> {
    data: PooledBitmapData<T>,
    pub width: usize,
    pub height: usize,
}

impl<T: Sized + ZeroValue + Copy> AsRef<[T]> for PooledBitmap<T> {
    fn as_ref(&self) -> &[T] {
        &self.data.data.as_ref().unwrap()[..]
    }
}

impl<T: Sized + ZeroValue + Copy> AsMut<[T]> for PooledBitmap<T> {
    fn as_mut(&mut self) -> &mut [T] {
        &mut self.data.data.as_mut().unwrap()[..]
    }
}

pub trait DataTypeBgra8x4: Sized + AsRef<[[u8; 4]]> + AsMut<[[u8; 4]]> {}
impl<T: Sized + AsRef<[[u8; 4]]> + AsMut<[[u8; 4]]>> DataTypeBgra8x4 for T {}

/// A Bgra8888 format bitmap
pub struct FrameBitmapBgraUnorm8x4<Data: DataTypeBgra8x4> {
    pub data: Data,
    pub width:  usize,
    pub height: usize,
}

pub trait DataTypeArgbUnormPacked2101010: Sized + AsRef<[u32]> {}
impl<T: Sized + AsRef<[u32]> + AsMut<[u32]>> DataTypeArgbUnormPacked2101010 for T {}

/// A Rgba1010102 format bitmap
pub struct FrameBitmapArgbUnormPacked2101010<Data: DataTypeArgbUnormPacked2101010> {
    pub data: Data,
    pub width:  usize,
    pub height: usize,
}

pub trait DataTypeRgbaF16x4: Sized + AsRef<[[f16; 4]]> {}
impl<T: Sized + AsRef<[[f16; 4]]> + AsMut<[[f16; 4]]>> DataTypeRgbaF16x4 for T {}

/// A RgbaF16x4 format bitmap
pub struct FrameBitmapRgbaF16x4<Data: DataTypeRgbaF16x4> {
    pub data: Data,
    pub width:  usize,
    pub height: usize,
}

/// The video range for a YCbCr format bitmap
pub enum VideoRange {
    /// Luma: [16, 240], Chroma: [0, 255]
    Video,
    /// Luma: [0, 255], Chroma: [0, 255]
    Full,
}

pub trait DataTypeLuma: Sized + AsRef<[u8]> {}
impl<T: Sized + AsRef<[u8]> + AsMut<[u8]>> DataTypeLuma for T {}

pub trait DataTypeChroma: Sized + AsRef<[[u8; 2]]> {}
impl<T: Sized + AsRef<[[u8; 2]]> + AsMut<[[u8; 2]]>> DataTypeChroma for T {}

/// A YCbCr image, corresponding to either V420 or F420 pixel formats.
/// 
/// Dual-planar, with luminance (Y) in one plane, and chrominance (CbCr) in another.
/// Note that each plane may have a different size, as with V420 format, where
/// the chroma plane is 2 by 2 blocks, but luma is per-pixel
pub struct FrameBitmapYCbCr<LumaData: DataTypeLuma, ChromaData: DataTypeChroma> {
    pub luma_data: LumaData,
    pub luma_width: usize,
    pub luma_height: usize,
    pub chroma_data: ChromaData,
    pub chroma_width: usize,
    pub chroma_height: usize,
    pub range: VideoRange,
}

/// A bitmap image of the selected format
pub enum FrameBitmap<DataBgra: DataTypeBgra8x4, DataArgbPacked: DataTypeArgbUnormPacked2101010, DataRgbaF16: DataTypeRgbaF16x4, DataLuma: DataTypeLuma, DataChroma: DataTypeChroma> {
    BgraUnorm8x4(FrameBitmapBgraUnorm8x4<DataBgra>),
    ArgbUnormPacked2101010(FrameBitmapArgbUnormPacked2101010<DataArgbPacked>),
    RgbaF16x4(FrameBitmapRgbaF16x4<DataRgbaF16>),
    YCbCr(FrameBitmapYCbCr<DataLuma, DataChroma>),
}

pub type BoxedSliceFrameBitmap = FrameBitmap<
    // Bgra8888
    Box<[[u8; 4]]>,
    // ArgbPacked2101010
    Box<[u32]>,
    // RgbaF16x4
    Box<[[f16; 4]]>,
    // Luma
    Box<[u8]>,
    // Chroma
    Box<[[u8; 2]]>
>;

pub type PooledFrameBitmap = FrameBitmap<
    // Bgra8888
    PooledBitmap<[u8; 4]>,
    // ArgbPacked2101010
    PooledBitmap<u32>,
    // RgbaF16x4
    PooledBitmap<[f16; 4]>,
    // Luma
    PooledBitmap<u8>,
    // Chroma
    PooledBitmap<[u8; 2]>,
>;

pub struct FrameBitmapPool {
    bgra_u8x4: Arc<BitmapPool<[u8; 4]>>,
    argb_packed_2101010: Arc<BitmapPool<u32>>,
    rgba_f16x4: Arc<BitmapPool<[f16; 4]>>,
    luma: Arc<BitmapPool<u8>>,
    chroma: Arc<BitmapPool<[u8; 2]>>,
}

impl FrameBitmapPool {
    pub fn new_with_initial_capacity(capacity: usize, initial_resolution: (usize, usize), max: usize, format: CapturePixelFormat) -> Self {
        Self {
            bgra_u8x4: BitmapPool::new(
                if format == CapturePixelFormat::Bgra8888 { capacity } else { 0 },
                max,
                initial_resolution
            ),
            argb_packed_2101010: BitmapPool::new(
                if format == CapturePixelFormat::Argb2101010 { capacity } else { 0 },
                max,
                initial_resolution
            ),
            rgba_f16x4: BitmapPool::new(
                0,
                max,
                initial_resolution
            ),
            luma: BitmapPool::new(
                if format == CapturePixelFormat::F420 || format == CapturePixelFormat::V420 { capacity } else { 0 },
                max,
                initial_resolution
            ),
            chroma: BitmapPool::new(
                if format == CapturePixelFormat::F420 || format == CapturePixelFormat::V420 { capacity } else { 0 },
                max,
                initial_resolution
            )
        }
    }

    pub fn new(max: usize) -> Self {
        Self {
            bgra_u8x4: BitmapPool::new(0, max, (0, 0)),
            argb_packed_2101010: BitmapPool::new(0, max, (0, 0)),
            rgba_f16x4: BitmapPool::new(0, max, (0, 0)),
            luma: BitmapPool::new(0, max, (0, 0)),
            chroma: BitmapPool::new(0, max, (0, 0)),
        }
    }

    pub fn free_pooled(&self) {
        self.bgra_u8x4.free_pooled();
        self.argb_packed_2101010.free_pooled();
        self.rgba_f16x4.free_pooled();
        self.luma.free_pooled();
        self.chroma.free_pooled();
    }
}

/// A video frame which can produce a bitmap
pub trait VideoFrameBitmap {
    /// Create a bitmap image from this frame. This usually involves a memory transfer from VRAM to system RAM,
    /// and is an expensive operation.
    fn get_bitmap(&self) -> Result<BoxedSliceFrameBitmap, VideoFrameBitmapError>;

    fn try_get_pooled_bitmap(&self, bitmap_pool: &FrameBitmapPool) -> Result<Option<PooledFrameBitmap>, VideoFrameBitmapError>;
    fn get_pooled_bitmap(&self, bitmap_pool: &FrameBitmapPool) -> Result<PooledFrameBitmap, VideoFrameBitmapError>;
}

#[derive(Clone, Debug)]
/// Represents an error while generating a frame bitmap
pub enum VideoFrameBitmapError {
    Other(String),
}

impl Display for VideoFrameBitmapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(error) => f.write_fmt(format_args!("VideoFrameBitmapError::Other(\"{}\")", error)),
        }
    }
}

impl Error for VideoFrameBitmapError {
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

#[derive(Copy, Clone)]
struct VideoFramePlanePtr {
    ptr: *const c_void,
    width: usize,
    height: usize,
    bytes_per_row: usize,
}

enum VideoFrameDataCopyPtrs {
    Bgra8888(VideoFramePlanePtr),
    ArgbPacked2101010(VideoFramePlanePtr),
    RgbaF16x4(VideoFramePlanePtr),
    F420{luma: VideoFramePlanePtr, chroma: VideoFramePlanePtr},
    V420{luma: VideoFramePlanePtr, chroma: VideoFramePlanePtr},
}

trait VideoFrameBitmapInternal {
    fn get_bitmap_internal<T>(&self, output_mapping: &impl Fn(VideoFrameDataCopyPtrs) -> Result<T, VideoFrameBitmapError>) -> Result<T, VideoFrameBitmapError>; 
}

impl VideoFrameBitmapInternal for VideoFrame {
    fn get_bitmap_internal<T>(&self, output_mapping: &impl Fn(VideoFrameDataCopyPtrs) -> Result<T, VideoFrameBitmapError>) -> Result<T, VideoFrameBitmapError> {
        #[cfg(target_os = "windows")]
        {
            let (width, height) = self.impl_video_frame.frame_size;
            match self.get_dx11_surface() {
                Err(WindowsDx11VideoFrameError::Other(x)) => Err(VideoFrameBitmapError::Other(x)),
                Ok((surface, pixel_format)) => {
                    let dxgi_format = match pixel_format {
                        DirectXPixelFormat::B8G8R8A8UIntNormalized => DXGI_FORMAT_B8G8R8A8_UNORM,
                        DirectXPixelFormat::R10G10B10A2UIntNormalized => DXGI_FORMAT_R10G10B10A2_UNORM,
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
                        let base_address = lock_gaurd.get_base_address().ok_or(VideoFrameBitmapError::Other("Failed to get base address of iosurface".into()))?;
                        
                        let plane_ptr = VideoFramePlanePtr {
                            ptr: base_address,
                            width,
                            height,
                            bytes_per_row: bpr
                        };

                        output_mapping(VideoFrameDataCopyPtrs::Bgra8888(plane_ptr))
                    },
                    Some(CVPixelFormat::V420) |
                    Some(CVPixelFormat::F420) => {

                        let luma_bpr = iosurface.get_bytes_per_row_of_plane(0);
                        let luma_height = iosurface.get_height_of_plane(0);
                        let luma_width = iosurface.get_width_of_plane(0);
                        let luma_base_address = lock_gaurd.get_base_address_of_plane(0).ok_or(VideoFrameBitmapError::Other("Failed to get base address of iosurface".into()))?;

                        let luma_plane_ptr = VideoFramePlanePtr {
                            ptr: luma_base_address,
                            width: luma_width,
                            height: luma_height,
                            bytes_per_row: luma_bpr,
                        };

                        let chroma_bpr = iosurface.get_bytes_per_row_of_plane(1);
                        let chroma_height = iosurface.get_height_of_plane(1);
                        let chroma_width = iosurface.get_width_of_plane(1);
                        let chroma_base_address = lock_gaurd.get_base_address_of_plane(1).ok_or(VideoFrameBitmapError::Other("Failed to get base address of iosurface".into()))?;

                        let chroma_plane_ptr = VideoFramePlanePtr {
                            ptr: chroma_base_address,
                            width: chroma_width,
                            height: chroma_height,
                            bytes_per_row: chroma_bpr,
                        };

                        if pixel_format == Some(CVPixelFormat::V420) {
                            output_mapping(VideoFrameDataCopyPtrs::V420 { luma: luma_plane_ptr, chroma: chroma_plane_ptr })
                        } else {
                            output_mapping(VideoFrameDataCopyPtrs::F420 { luma: luma_plane_ptr, chroma: chroma_plane_ptr })
                        }
                    },
                    _ => Err(VideoFrameBitmapError::Other("Unknown pixel format on iosurface".to_string()))
                }
            } else {
                Err(VideoFrameBitmapError::Other("Failed to lock iosurface".to_string()))
            }
        }
    }
}

fn copy_boxed_slice_plane<T: Sized + Copy + Pod + ZeroValue>(plane_ptr: VideoFramePlanePtr) -> Box<[T]> {
    let mut image_data = vec![T::zero_value(); plane_ptr.width * plane_ptr.height];
    let src_slice = unsafe { std::slice::from_raw_parts(plane_ptr.ptr as *const u8, plane_ptr.bytes_per_row * plane_ptr.height) };
    for y in 0..plane_ptr.height {
        let source_slice = bytemuck::cast_slice::<_, T>(&src_slice[(plane_ptr.bytes_per_row * y)..(plane_ptr.bytes_per_row * y + std::mem::size_of::<T>() * plane_ptr.width)]);
        image_data[(plane_ptr.width * y)..(plane_ptr.width * y + plane_ptr.width)].copy_from_slice(source_slice);
    }
    image_data.into_boxed_slice()
}

fn copy_pooled_plane<T: Sized + Copy + Pod + ZeroValue>(plane_ptr: VideoFramePlanePtr, pool: &Arc<BitmapPool<T>>) -> PooledBitmap<T> {
    let mut bitmap = pool.get_bitmap((plane_ptr.width, plane_ptr.height));
    let src_slice = unsafe { std::slice::from_raw_parts(plane_ptr.ptr as *const u8, plane_ptr.bytes_per_row * plane_ptr.height) };
    for y in 0..plane_ptr.height {
        let source_slice = bytemuck::cast_slice::<_, T>(&src_slice[(plane_ptr.bytes_per_row * y)..(plane_ptr.bytes_per_row * y + std::mem::size_of::<T>() * plane_ptr.width)]);
        AsMut::as_mut(&mut bitmap)[(plane_ptr.width * y)..(plane_ptr.width * y + plane_ptr.width)].copy_from_slice(source_slice);
    }
    bitmap
}

fn try_copy_pooled_plane<T: Sized + Copy + Pod + ZeroValue>(plane_ptr: VideoFramePlanePtr, pool: &Arc<BitmapPool<T>>) -> Option<PooledBitmap<T>> {
    let mut bitmap = pool.try_get_bitmap((plane_ptr.width, plane_ptr.height))?;
    let src_slice = unsafe { std::slice::from_raw_parts(plane_ptr.ptr as *const u8, plane_ptr.bytes_per_row * plane_ptr.height) };
    for y in 0..plane_ptr.height {
        let source_slice = bytemuck::cast_slice::<_, T>(&src_slice[(plane_ptr.bytes_per_row * y)..(plane_ptr.bytes_per_row * y + std::mem::size_of::<T>() * plane_ptr.width)]);
        AsMut::as_mut(&mut bitmap)[(plane_ptr.width * y)..(plane_ptr.width * y + plane_ptr.width)].copy_from_slice(source_slice);
    }
    Some(bitmap)
}

impl VideoFrameBitmap for VideoFrame {
    fn get_bitmap(&self) -> Result<BoxedSliceFrameBitmap, VideoFrameBitmapError> {
        self.get_bitmap_internal::<BoxedSliceFrameBitmap>(&|copy_ptrs| {
            match copy_ptrs {
                VideoFrameDataCopyPtrs::Bgra8888(bgra_plane_ptr) => {
                    Ok(BoxedSliceFrameBitmap::BgraUnorm8x4(FrameBitmapBgraUnorm8x4 {
                        data: copy_boxed_slice_plane(bgra_plane_ptr),
                        width: bgra_plane_ptr.width,
                        height: bgra_plane_ptr.height,
                    }))
                },
                VideoFrameDataCopyPtrs::ArgbPacked2101010(argb_plane_ptr) => {
                    Ok(BoxedSliceFrameBitmap::ArgbUnormPacked2101010(FrameBitmapArgbUnormPacked2101010 {
                        data: copy_boxed_slice_plane(argb_plane_ptr),
                        width: argb_plane_ptr.width,
                        height: argb_plane_ptr.height,
                    }))
                },
                VideoFrameDataCopyPtrs::F420 { luma: luma_plane_ptr, chroma: chroma_plane_ptr } => {
                    Ok(BoxedSliceFrameBitmap::YCbCr(FrameBitmapYCbCr {
                        luma_data: copy_boxed_slice_plane(luma_plane_ptr),
                        luma_width: luma_plane_ptr.width,
                        luma_height: luma_plane_ptr.height,
                        chroma_data: copy_boxed_slice_plane(chroma_plane_ptr),
                        chroma_width: chroma_plane_ptr.width,
                        chroma_height: chroma_plane_ptr.height,
                        range: VideoRange::Full
                    }))
                },
                VideoFrameDataCopyPtrs::V420 { luma: luma_plane_ptr, chroma: chroma_plane_ptr } => {
                    Ok(BoxedSliceFrameBitmap::YCbCr(FrameBitmapYCbCr {
                        luma_data: copy_boxed_slice_plane(luma_plane_ptr),
                        luma_width: luma_plane_ptr.width,
                        luma_height: luma_plane_ptr.height,
                        chroma_data: copy_boxed_slice_plane(chroma_plane_ptr),
                        chroma_width: chroma_plane_ptr.width,
                        chroma_height: chroma_plane_ptr.height,
                        range: VideoRange::Video
                    }))
                },
                VideoFrameDataCopyPtrs::RgbaF16x4(rgba_plane_ptr) => {
                    Ok(BoxedSliceFrameBitmap::RgbaF16x4(FrameBitmapRgbaF16x4 {
                        data: copy_boxed_slice_plane(rgba_plane_ptr),
                        width: rgba_plane_ptr.width,
                        height: rgba_plane_ptr.height,
                    }))
                }
            }
        })
    }

    fn get_pooled_bitmap(&self, bitmap_pool: &FrameBitmapPool) -> Result<PooledFrameBitmap, VideoFrameBitmapError> {
        self.get_bitmap_internal::<PooledFrameBitmap>(&|copy_ptrs| {
            match copy_ptrs {
                VideoFrameDataCopyPtrs::Bgra8888(bgra_plane_ptr) => {
                    Ok(PooledFrameBitmap::BgraUnorm8x4(FrameBitmapBgraUnorm8x4 {
                        data: copy_pooled_plane(bgra_plane_ptr, &bitmap_pool.bgra_u8x4),
                        width: bgra_plane_ptr.width,
                        height: bgra_plane_ptr.height,
                    }))
                },
                VideoFrameDataCopyPtrs::ArgbPacked2101010(argb_plane_ptr) => {
                    Ok(PooledFrameBitmap::ArgbUnormPacked2101010(FrameBitmapArgbUnormPacked2101010 {
                        data: copy_pooled_plane(argb_plane_ptr, &bitmap_pool.argb_packed_2101010),
                        width: argb_plane_ptr.width,
                        height: argb_plane_ptr.height,
                    }))
                },
                VideoFrameDataCopyPtrs::F420 { luma: luma_plane_ptr, chroma: chroma_plane_ptr } => {
                    Ok(PooledFrameBitmap::YCbCr(FrameBitmapYCbCr {
                        luma_data: copy_pooled_plane(luma_plane_ptr, &bitmap_pool.luma),
                        luma_width: luma_plane_ptr.width,
                        luma_height: luma_plane_ptr.height,
                        chroma_data: copy_pooled_plane(chroma_plane_ptr, &bitmap_pool.chroma),
                        chroma_width: chroma_plane_ptr.width,
                        chroma_height: chroma_plane_ptr.height,
                        range: VideoRange::Full
                    }))
                },
                VideoFrameDataCopyPtrs::V420 { luma: luma_plane_ptr, chroma: chroma_plane_ptr } => {
                    Ok(PooledFrameBitmap::YCbCr(FrameBitmapYCbCr {
                        luma_data: copy_pooled_plane(luma_plane_ptr, &bitmap_pool.luma),
                        luma_width: luma_plane_ptr.width,
                        luma_height: luma_plane_ptr.height,
                        chroma_data: copy_pooled_plane(chroma_plane_ptr, &bitmap_pool.chroma),
                        chroma_width: chroma_plane_ptr.width,
                        chroma_height: chroma_plane_ptr.height,
                        range: VideoRange::Video
                    }))
                },
                VideoFrameDataCopyPtrs::RgbaF16x4(rgba_plane_ptr) => {
                    Ok(PooledFrameBitmap::RgbaF16x4(FrameBitmapRgbaF16x4 {
                        data: copy_pooled_plane(rgba_plane_ptr, &bitmap_pool.rgba_f16x4),
                        width: rgba_plane_ptr.width,
                        height: rgba_plane_ptr.height,
                    }))
                }
            }
        })
    }

    fn try_get_pooled_bitmap(&self, bitmap_pool: &FrameBitmapPool) -> Result<Option<PooledFrameBitmap>, VideoFrameBitmapError> {
        self.get_bitmap_internal::<Option<PooledFrameBitmap>>(&|copy_ptrs| {
            match copy_ptrs {
                VideoFrameDataCopyPtrs::Bgra8888(bgra_plane_ptr) => {
                    if let Some(data) = try_copy_pooled_plane(bgra_plane_ptr, &bitmap_pool.bgra_u8x4) {
                        Ok(Some(PooledFrameBitmap::BgraUnorm8x4(FrameBitmapBgraUnorm8x4 {
                            data,
                            width: bgra_plane_ptr.width,
                            height: bgra_plane_ptr.height,
                        })))
                    } else {
                        Ok(None)
                    }
                },
                VideoFrameDataCopyPtrs::ArgbPacked2101010(argb_plane_ptr) => {
                    if let Some(data) = try_copy_pooled_plane(argb_plane_ptr, &bitmap_pool.argb_packed_2101010) {
                        Ok(Some(PooledFrameBitmap::ArgbUnormPacked2101010(FrameBitmapArgbUnormPacked2101010 {
                            data,
                            width: argb_plane_ptr.width,
                            height: argb_plane_ptr.height,
                        })))
                    } else {
                        Ok(None)
                    }
                },
                VideoFrameDataCopyPtrs::F420 { luma: luma_plane_ptr, chroma: chroma_plane_ptr } => {
                    if let (Some(luma_data), Some(chroma_data)) = (try_copy_pooled_plane(luma_plane_ptr, &bitmap_pool.luma), try_copy_pooled_plane(chroma_plane_ptr, &bitmap_pool.chroma)) {
                        Ok(Some(PooledFrameBitmap::YCbCr(FrameBitmapYCbCr {
                            luma_data,
                            luma_width: luma_plane_ptr.width,
                            luma_height: luma_plane_ptr.height,
                            chroma_data,
                            chroma_width: chroma_plane_ptr.width,
                            chroma_height: chroma_plane_ptr.height,
                            range: VideoRange::Full
                        })))
                    } else {
                        Ok(None)
                    }
                    
                },
                VideoFrameDataCopyPtrs::V420 { luma: luma_plane_ptr, chroma: chroma_plane_ptr } => {
                    if let (Some(luma_data), Some(chroma_data)) = (try_copy_pooled_plane(luma_plane_ptr, &bitmap_pool.luma), try_copy_pooled_plane(chroma_plane_ptr, &bitmap_pool.chroma)) {
                        Ok(Some(PooledFrameBitmap::YCbCr(FrameBitmapYCbCr {
                            luma_data,
                            luma_width: luma_plane_ptr.width,
                            luma_height: luma_plane_ptr.height,
                            chroma_data,
                            chroma_width: chroma_plane_ptr.width,
                            chroma_height: chroma_plane_ptr.height,
                            range: VideoRange::Video
                        })))
                    } else {
                        Ok(None)
                    }
                },
                VideoFrameDataCopyPtrs::RgbaF16x4(rgba_plane_ptr) => {
                    if let Some(data) = try_copy_pooled_plane(rgba_plane_ptr, &bitmap_pool.rgba_f16x4) {
                        Ok(Some(PooledFrameBitmap::RgbaF16x4(FrameBitmapRgbaF16x4 {
                            data,
                            width: rgba_plane_ptr.width,
                            height: rgba_plane_ptr.height,
                        })))
                    } else {
                        Ok(None)
                    }
                }
            }
        })
    }
}


