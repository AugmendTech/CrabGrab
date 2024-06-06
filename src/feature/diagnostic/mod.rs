#![allow(unused)]

#[cfg(target_os = "windows")]
use windows::{Graphics::DirectX::DirectXPixelFormat, Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_0, Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_11_1, Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_12_0, Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_12_1, Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_12_2, Win32::Graphics::Direct3D::D3D_FEATURE_LEVEL_1_0_CORE};

#[cfg(target_os = "macos")]
use crate::platform::platform_impl::objc_wrap::{IOSurface, CGRect, NSDictionary, NSNumber, NSString, SCStreamFrameInfoBoundingRect, SCStreamFrameInfoContentRect, SCStreamFrameInfoContentScale, SCStreamFrameInfoScaleFactor, SCStreamFrameInfoStatus};

#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
pub struct FrameIOSurfaceComponentInfo {
    name: String,
    data_type: String,
    bit_depth: usize,
    bit_offset: usize,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
pub struct FrameIOSurfacePlaneInfo {
    width: usize,
    height: usize,
    bytes_per_row: usize,
    components: Vec<FrameIOSurfaceComponentInfo>,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
pub struct FrameIOSurfaceInfo {
    pixel_format: String,
    width: usize,
    height: usize,
    bytes_per_row: usize,
    planes: Vec<FrameIOSurfacePlaneInfo>,
}

#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
pub struct FrameDx11SurfaceInfo {
    pixel_format: String,
    width: usize,
    height: usize,
}

/// Diagnostic information about a capture frame
#[derive(Debug, Clone)]
pub struct FrameDiagnostic {
    #[cfg(target_os = "macos")]
    pub iosurface_info: Option<FrameIOSurfaceInfo>,
    #[cfg(target_os = "macos")]
    pub info_dictionary: Vec<(String, String)>,
    #[cfg(target_os = "windows")]
    pub dx11_surface_info: FrameDx11SurfaceInfo,
    #[cfg(target_os = "windows")]
    pub dx_feature_level: String,
}

/// A frame that supports gathering diagnostic information
pub trait FrameDiagnosticExt {
    fn diagnostic(&self) -> FrameDiagnostic;
}

#[cfg(target_os = "macos")]
fn get_iosurface_info(iosurface: &IOSurface) -> FrameIOSurfaceInfo {
    let pixel_format = format!("{:?}", iosurface.get_pixel_format());
    let width = iosurface.get_width();
    let height = iosurface.get_height();
    let bytes_per_row = iosurface.get_bytes_per_row();
    let plane_count = iosurface.get_plane_count();
    let mut planes = Vec::new();
    for plane in 0..plane_count {
        let width = iosurface.get_width_of_plane(plane);
        let height = iosurface.get_height_of_plane(plane);
        let bytes_per_row = iosurface.get_bytes_per_row_of_plane(plane);
        let component_count = iosurface.get_plane_component_count(plane);
        let mut components = Vec::new();
        for component in 0..component_count {
            components.push(
                FrameIOSurfaceComponentInfo {
                    name: format!("{:?}", iosurface.get_plane_component_name(plane, component)),
                    data_type: format!("{:?}", iosurface.get_plane_component_type(plane, component)),
                    bit_depth: iosurface.get_plane_component_bit_depth(plane, component),
                    bit_offset: iosurface.get_plane_component_bit_offset(plane, component)
                }
            )
        }
        planes.push(FrameIOSurfacePlaneInfo {
            width,
            height,
            bytes_per_row,
            components
        })
    }
    FrameIOSurfaceInfo {
        pixel_format,
        width,
        height,
        bytes_per_row,
        planes
    }
}

impl FrameDiagnosticExt for crate::prelude::VideoFrame {
    fn diagnostic(&self) -> FrameDiagnostic {
        #[cfg(target_os = "macos")]
        {
            match &self.impl_video_frame {
                crate::platform::platform_impl::ImplVideoFrame::SCStream(sc_stream_frame) => {
                    unsafe { 
                        let mut info_dictionary = Vec::new();
                        let info_dict = sc_stream_frame.get_info_dict();
                        let bounding_rect_dict_raw = info_dict.get_value(SCStreamFrameInfoBoundingRect as *const _);
                        if !bounding_rect_dict_raw.is_null() {
                            let bounding_rect_dict = NSDictionary::from_ref_unretained(bounding_rect_dict_raw);
                            let bounding_rect = CGRect::create_from_dictionary_representation(&bounding_rect_dict);
                            info_dictionary.push((NSString::from_ref_unretained(SCStreamFrameInfoBoundingRect).as_string(), format!("{:?}", bounding_rect)));
                        }
                        let content_rect_dict_raw = info_dict.get_value(SCStreamFrameInfoContentRect as *const _);
                        if !content_rect_dict_raw.is_null() {
                            let content_rect_dict = NSDictionary::from_ref_unretained(content_rect_dict_raw);
                            let content_rect = CGRect::create_from_dictionary_representation(&content_rect_dict);
                            info_dictionary.push((NSString::from_ref_unretained(SCStreamFrameInfoContentRect).as_string(), format!("{:?}", content_rect)));
                        }
                        let status_nsnumber_raw = info_dict.get_value(SCStreamFrameInfoStatus as *const _);
                        if !status_nsnumber_raw.is_null() {
                            let status_nsnumber = NSNumber::from_id_unretained(status_nsnumber_raw as _);
                            info_dictionary.push((NSString::from_ref_unretained(SCStreamFrameInfoStatus).as_string(), format!("{:?}", status_nsnumber.as_i32())));
                        }
                        let content_scale_nsnumber_raw = info_dict.get_value(SCStreamFrameInfoContentScale as *const _);
                        if !content_scale_nsnumber_raw.is_null() {
                            let content_scale_nsnumber = NSNumber::from_id_unretained(content_scale_nsnumber_raw as _);
                            info_dictionary.push((NSString::from_ref_unretained(SCStreamFrameInfoContentScale).as_string(), format!("{:?}", content_scale_nsnumber.as_f64())));
                        }
                        let scale_factor_nsnumber_raw = info_dict.get_value(SCStreamFrameInfoScaleFactor as *const _);
                        if !scale_factor_nsnumber_raw.is_null() {
                            let scale_factor_nsnumber = NSNumber::from_id_unretained(scale_factor_nsnumber_raw as _);
                            info_dictionary.push((NSString::from_ref_unretained(SCStreamFrameInfoScaleFactor).as_string(), format!("{:?}", scale_factor_nsnumber.as_f64())));
                        }
                        let iosurface_info = if let Some(iosurface) = sc_stream_frame.sample_buffer.get_image_buffer().map(|image_buffer| image_buffer.get_iosurface()).flatten() {
                            Some(get_iosurface_info(&iosurface))
                        } else {
                            None
                        };
                        FrameDiagnostic {
                            iosurface_info,
                            info_dictionary
                        }
                    }
                },
                crate::platform::platform_impl::ImplVideoFrame::CGDisplayStream(_) =>  {
                    todo!()
                },
            }
        }
        #[cfg(target_os = "windows")]
        {
            let pixel_format = match self.impl_video_frame.pixel_format {
                DirectXPixelFormat::B8G8R8A8UIntNormalized => "B8G8R8A8UIntNormalized",
                DirectXPixelFormat::R10G10B10A2UIntNormalized => "R10G10B10A2UIntNormalized",
                _ => "unknown"
            }.to_string();
            let (width, height) = self.impl_video_frame.frame_size;
            let dx_feature_level = match unsafe { self.impl_video_frame.device.GetFeatureLevel() } {
                D3D_FEATURE_LEVEL_11_0 => "11_0",
                D3D_FEATURE_LEVEL_11_1 => "11_1",
                D3D_FEATURE_LEVEL_12_0 => "12_0",
                D3D_FEATURE_LEVEL_12_1 => "12_1",
                D3D_FEATURE_LEVEL_12_2 => "12_2",
                D3D_FEATURE_LEVEL_1_0_CORE => "1_0_CORE",
                _ => "unknown"
            }.to_string();
            FrameDiagnostic {
                dx11_surface_info: FrameDx11SurfaceInfo {
                    pixel_format,
                    width,
                    height,
                },
                dx_feature_level
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct StreamDiagnostic {
    //#[cfg(target_os = "macos")]

}
