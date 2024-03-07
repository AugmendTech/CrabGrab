use std::{ptr::NonNull, sync::Arc};

use crate::{prelude::{Capturable, CaptureConfig, CapturePixelFormat, StreamCreateError, StreamError, StreamEvent, StreamStopError}, util::Rect};

use windows::{core::{IUnknown, HSTRING}, Graphics::{Capture::{Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureAccess, GraphicsCaptureAccessKind, GraphicsCaptureItem, GraphicsCaptureSession}, DirectX::DirectXPixelFormat, SizeInt32}, Security::Authorization::AppCapabilityAccess::{AppCapability, AppCapabilityAccessStatus}, Win32::{Graphics::{Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0}, Direct3D11::{D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION}, Dxgi::{CreateDXGIFactory, IDXGIAdapter, IDXGIFactory}}, System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop}};

use super::frame;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WindowsPixelFormat {
    Bgra8888,
}

#[derive(Clone, Debug)]
pub struct WindowsAudioCaptureConfig {}

impl WindowsAudioCaptureConfig {
    pub fn new() -> Self {
        Self {
        }
    }
}

pub trait WindowsAudioCaptureConfigExt {

}

impl WindowsAudioCaptureConfigExt for CaptureConfig {

}

#[derive(Clone, Debug)]
pub struct WindowsCaptureConfig {
    dxgi_adapter: Option<IDXGIAdapter>,
    d3d11_device: Option<ID3D11Device>,
}

impl WindowsCaptureConfig {
    pub fn new(rect: Rect) -> Self {
        Self {
            dxgi_adapter: None,
            d3d11_device: None,
        }
    }
}

pub trait WindowsCaptureConfigExt {
    fn with_dxgi_adapter(self, dxgi_adapter: IDXGIAdapter) -> Self;
    fn with_d3d11_device(self, d3d11_device: ID3D11Device) -> Self;
}

impl WindowsCaptureConfigExt for CaptureConfig {
    fn with_dxgi_adapter(self, dxgi_adapter: IDXGIAdapter) -> Self {
        Self {
            impl_capture_config: WindowsCaptureConfig {
                dxgi_adapter: Some(dxgi_adapter),
                ..self.impl_capture_config
            },
            ..self
        }
    }

    fn with_d3d11_device(self, d3d11_device: ID3D11Device) -> Self {
        Self {
            impl_capture_config: WindowsCaptureConfig {
                d3d11_device: Some(d3d11_device),
                ..self.impl_capture_config
            },
            ..self
        }
    }
}

pub struct WindowsCaptureStream {
    dxgi_adapter: IDXGIAdapter,
    d3d11_device: ID3D11Device,

}

impl WindowsCaptureStream {
    pub fn supported_pixel_formats() -> &'static [CapturePixelFormat] {
        &[
            CapturePixelFormat::Bgra8888,
            CapturePixelFormat::Argb2101010,
        ]
    }

    pub fn check_access(borderless: bool) -> bool {
        let graphics_capture_capability = HSTRING::from("graphicsCaptureProgrammatic");
        let programmatic_access = AppCapability::Create(&graphics_capture_capability).map(|capability| {
            match capability.CheckAccess() {
                Ok(AppCapabilityAccessStatus::Allowed) => true,
                _ => false,
            }
        }).unwrap_or(true);
        let borderless_graphics_capture_capability = HSTRING::from("graphicsCaptureWithoutBorder");
        let borderless_access = AppCapability::Create(&borderless_graphics_capture_capability).map(|capability| {
            match capability.CheckAccess() {
                Ok(AppCapabilityAccessStatus::Allowed) => true,
                _ => false,
            }
        }).unwrap_or(true);
        programmatic_access && (!borderless || borderless_access)
    }

    pub async fn request_access(borderless: bool) -> bool {
        let access_kind = if borderless {
            GraphicsCaptureAccessKind::Borderless
        } else {
            GraphicsCaptureAccessKind::Programmatic
        };
        match GraphicsCaptureAccess::RequestAccessAsync(access_kind) {
            Ok(access_future) => match access_future.await {
                Ok(AppCapabilityAccessStatus::Allowed) => true,
                _ => false
            },
            _ => false,
        }
    }

    fn crate_d3d11_device(dxgi_adapter: IDXGIAdapter) -> Result<ID3D11Device, StreamCreateError> {
        unsafe {
            let mut d3d11_device = None;
            let d3d11_device_result = D3D11CreateDevice(
                Some(&dxgi_adapter),
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut d3d11_device as *mut _),
                None,
                None
            );
            match d3d11_device_result {
                Ok(_) => d3d11_device.map_or_else(|| Err(StreamCreateError::Other("Failed to create ID3D11Device".into())), |x| Ok(x)),
                Err(e) => Err(StreamCreateError::Other(format!("Failed to create d3d11 device")))
                ,
            }
        }
    }

    pub fn new(config: CaptureConfig, callback: Box<impl FnMut(Result<StreamEvent, StreamError>) + Send + 'static>) -> Result<Self, StreamCreateError> {
        let pixel_format = match config.pixel_format {
            CapturePixelFormat::Bgra8888 => DirectXPixelFormat::B8G8R8A8Typeless,
            CapturePixelFormat::Argb2101010 => DirectXPixelFormat::R10G10B10A2Typeless,
            _ => return Err(StreamCreateError::UnsupportedPixelFormat),
        };
        let interop: IGraphicsCaptureItemInterop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
            .map_err(|_| StreamCreateError::Other("Failed to create IGraphicsCaptureInterop factory".into()))?;
        let graphics_capture_item: GraphicsCaptureItem = unsafe {
            match config.target {
                Capturable::Window(window) =>
                    interop.CreateForWindow(window.impl_capturable_window.0)
                        .map_err(|_| StreamCreateError::Other("Failed to create graphics capture item from HWND".into()))?,
                Capturable::Display(display) => 
                    interop.CreateForMonitor(display.impl_capturable_display.0)
                        .map_err(|_| StreamCreateError::Other("Failed to create graphics capture item from HMONITOR".into()))?,
            }
        };
        let d3d11_device = match (config.impl_capture_config.dxgi_adapter, config.impl_capture_config.d3d11_device) {
            (_, Some(d3d11_device)) => d3d11_device,
            (Some(dxgi_adapter), None) => Self::crate_d3d11_device(dxgi_adapter)?,
            (None, None) => {
                let dxgi_factory: IDXGIFactory = unsafe { CreateDXGIFactory()
                    .map_err(|_| StreamCreateError::Other("Failed to create IDXGIAdapter factory".into())) }?;
                let dxgi_adapter = unsafe { dxgi_factory.EnumAdapters(0) }
                    .map_err(|_| StreamCreateError::Other("Failed to enumerate IDXGIAdapter".into()))?;
                Self::crate_d3d11_device(dxgi_adapter)?
            }
        };
        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            d3d11_device.into(),
            pixel_format,
            config.buffer_count as i32,
            SizeInt32 { Width: config.output_size.width as i32, Height: config.output_size.height as i32 },
        ).map_err(|_| StreamCreateError::Other("Failed to create Direct3D11CaptureFramePool".into()))?;
        let capture_session = frame_pool.CreateCaptureSession(&graphics_capture_item)
            .map_err(|_| StreamCreateError::Other("Failed to create GraphicsCaptureSession".into()))?;
    }

    pub fn stop(&self) -> Result<(), StreamStopError> {
        Ok(())
    }
}
