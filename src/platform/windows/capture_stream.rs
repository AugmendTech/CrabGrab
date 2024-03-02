use crate::{prelude::{Capturable, CaptureConfig, CapturePixelFormat, StreamCreateError, StreamError, StreamEvent, StreamStopError}, util::Rect};

use windows::{core::HSTRING, Graphics::Capture::{Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureAccess, GraphicsCaptureAccessKind, GraphicsCaptureItem, GraphicsCaptureSession}, Security::Authorization::AppCapabilityAccess::{AppCapability, AppCapabilityAccessStatus}, Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop};

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
    d3d11_device: Option<()>,
}

impl WindowsCaptureConfig {
    pub fn new(rect: Rect) -> Self {
        Self {
            d3d11_device: None,
        }
    }
}

pub trait WindowsCaptureConfigExt {
    fn set_d3d11_device(&mut self, device: ());
}

impl WindowsCaptureConfigExt for CaptureConfig {
    fn set_d3d11_device(&mut self, device: ()) {
        todo!()
    }
}

pub struct WindowsCaptureStream {
    
}

impl WindowsCaptureStream {
    pub fn supported_pixel_formats() -> &'static [CapturePixelFormat] {
        &[
            CapturePixelFormat::Bgra8888
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

    pub fn new(config: CaptureConfig, callback: Box<impl FnMut(Result<StreamEvent, StreamError>) + Send + 'static>) -> Result<Self, StreamCreateError> {
        /*
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
                .map_err(|_| format!("Failed to create graphics capture item interop"))?;
            let window_capture = 
                interop.CreateForWindow::<HWND, GraphicsCaptureItem>(window)
                    .map_err(|error| format!("Failed to create graphics capture item - {}", error.message()))?;
         */
        let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
            .map_err(|_| StreamCreateError::Other("Failed to create IGraphicsCaptureInterop factory".into()))?;
        match config.target {
            Capturable::Window(window) => {
                unsafe {
                    let capture_item = interop.CreateForWindow(window.impl_capturable_window.0)
                        .map_err(|_| StreamCreateError::Other("Failed to create graphics capture item from HWND".into()))?;
                    todo!()
                }
            },
            Capturable::Display(display) => {
                Err(StreamCreateError::Other("Windows display capture unimplemented".into()))
            }
        }
    }

    pub fn stop(&self) -> Result<(), StreamStopError> {
        Ok(())
    }
}
