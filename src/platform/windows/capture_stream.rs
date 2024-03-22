use std::sync::{atomic::{self, AtomicBool, AtomicU64}, Arc};

use crate::{prelude::{Capturable, CaptureConfig, CapturePixelFormat, StreamCreateError, StreamError, StreamEvent, StreamStopError, VideoFrame}, util::Rect};

use parking_lot::Mutex;
use windows::{core::{ComInterface, IInspectable, HSTRING}, Foundation::TypedEventHandler, Graphics::{Capture::{Direct3D11CaptureFramePool, GraphicsCaptureAccess, GraphicsCaptureAccessKind, GraphicsCaptureItem, GraphicsCaptureSession}, DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat}, SizeInt32}, Security::Authorization::AppCapabilityAccess::{AppCapability, AppCapabilityAccessStatus}, Win32::{Graphics::{Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0}, Direct3D11::{D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION}, Dxgi::{CreateDXGIFactory, IDXGIAdapter, IDXGIDevice, IDXGIFactory}}, System::{Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED}, WinRT::{Direct3D11::CreateDirect3D11DeviceFromDXGIDevice, Graphics::Capture::IGraphicsCaptureItemInterop}}}};

use super::frame::WindowsVideoFrame;

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
    pub(crate) dxgi_adapter: IDXGIAdapter,
    pub(crate) dxgi_device: IDXGIDevice,
    pub(crate) d3d11_device: ID3D11Device,
    pub(crate) frame_pool: Direct3D11CaptureFramePool,
    pub(crate) capture_session: GraphicsCaptureSession,
    should_couninit: bool,
    shared_handler_data: Arc<SharedHandlerData>,
}

struct SharedHandlerData {
    callback: Mutex<Box<dyn FnMut(Result<StreamEvent, StreamError>) + Send + 'static>>,
    closed: AtomicBool,
    frame_id_counter: AtomicU64,
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

    fn create_d3d11_device(dxgi_adapter: IDXGIAdapter) -> Result<(IDXGIAdapter, ID3D11Device), StreamCreateError> {
        unsafe {
            let mut d3d11_device = None;
            let d3d11_device_result = D3D11CreateDevice(
                Some(&dxgi_adapter),
                D3D_DRIVER_TYPE_UNKNOWN,
                None,
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&[D3D_FEATURE_LEVEL_11_0]),
                D3D11_SDK_VERSION,
                Some(&mut d3d11_device as *mut _),
                None,
                None
            );
            match d3d11_device_result {
                Ok(_) => d3d11_device.map_or_else(|| Err(StreamCreateError::Other("Failed to create ID3D11Device".into())), |x| Ok((dxgi_adapter, x))),
                Err(e) => Err(StreamCreateError::Other(format!("Failed to create d3d11 device")))
                ,
            }
        }
    }

    pub fn new(config: CaptureConfig, callback: Box<impl FnMut(Result<StreamEvent, StreamError>) + Send + 'static>) -> Result<Self, StreamCreateError> {
        let should_couninit = unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).is_ok()
        };
        
        let pixel_format = match config.pixel_format {
            CapturePixelFormat::Bgra8888 => DirectXPixelFormat::B8G8R8A8UIntNormalized,
            CapturePixelFormat::Argb2101010 => DirectXPixelFormat::R10G10B10A2UIntNormalized,
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

        let (dxgi_adapter, d3d11_device) = match (config.impl_capture_config.dxgi_adapter, config.impl_capture_config.d3d11_device) {
            (_, Some(d3d11_device)) => {
                let dxgi_adapter = d3d11_device.cast().map_err(|_| StreamCreateError::Other("Failed to create IDXGIAdapter from ID3D11Device".into()))?;
                (dxgi_adapter, d3d11_device)
            },
            (Some(dxgi_adapter), None) => Self::create_d3d11_device(dxgi_adapter)?,
            (None, None) => {
                let dxgi_factory: IDXGIFactory = unsafe { CreateDXGIFactory()
                    .map_err(|_| StreamCreateError::Other("Failed to create IDXGIAdapter factory".into())) }?;
                let dxgi_adapter = unsafe { dxgi_factory.EnumAdapters(0) }
                    .map_err(|_| StreamCreateError::Other("Failed to enumerate IDXGIAdapter".into()))?;
                Self::create_d3d11_device(dxgi_adapter)?
            }
        };

        let dxgi_device: IDXGIDevice = d3d11_device.clone().cast()
            .map_err(|_| StreamCreateError::Other("Failed to cast ID3D11Device to IDXGIDevice".into()))?;
        let direct3d_device_iinspectible = unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device) }
            .map_err(|_| StreamCreateError::Other("Failed to create IDirect3DDevice from IDXGIDevice".into()))?;
        let direct3d_device: IDirect3DDevice = direct3d_device_iinspectible.cast()
            .map_err(|_| StreamCreateError::Other("Failed to cast IInspectible to IDirect3DDevice".into()))?;

        let callback_direct3d_device = d3d11_device.clone();

        let (width, height) = ((config.output_size.width + 0.1) as usize, (config.output_size.height + 0.1) as usize);

        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &direct3d_device,
            pixel_format,
            config.buffer_count as i32,
            SizeInt32 { Width: width as i32, Height: height as i32 },
        ).map_err(|e| StreamCreateError::Other(format!("Failed to create Direct3D11CaptureFramePool: {}", e.to_string())))?;

        let shared_handler_data = Arc::new(
            SharedHandlerData {
                callback: Mutex::new(callback),
                closed: AtomicBool::new(false),
                frame_id_counter: AtomicU64::new(0),
            }
        );

        let close_handler_data = shared_handler_data.clone();
        let frame_handler_data = shared_handler_data.clone();

        let close_handler = TypedEventHandler::new(move |_, _| {
            let alread_closed = close_handler_data.closed.fetch_and(true, atomic::Ordering::AcqRel);
            if !alread_closed {
                let mut callback = close_handler_data.callback.lock();
                (*callback)(Ok(StreamEvent::End));
            }
            Ok(())
        });

        let frame_handler = TypedEventHandler::new(move |frame_pool: &Option<Direct3D11CaptureFramePool>, _: &Option<IInspectable>| {
            if frame_pool.is_none() {
                return Ok(());
            }
            let frame_pool = frame_pool.as_ref().unwrap();
            if frame_handler_data.closed.load(atomic::Ordering::Acquire) {
                return Ok(());
            }
            let mut callback = frame_handler_data.callback.lock();
            let frame = match frame_pool.TryGetNextFrame() {
                Ok(frame) => frame,
                Err(e) => {
                    (*callback)(Err(StreamError::Other(format!("Failed to capture frame: {}", e.to_string()))));
                    return Ok(());
                }
            };
            let frame_id = frame_handler_data.frame_id_counter.fetch_add(1, atomic::Ordering::AcqRel);
            let impl_video_frame = WindowsVideoFrame {
                device: callback_direct3d_device.clone(),
                frame,
                frame_id,
                frame_size: (width, height),
                pixel_format
            };
            let video_frame = VideoFrame {
                impl_video_frame
            };
            (*callback)(Ok(StreamEvent::Video(video_frame)));
            Ok(())
        });

        frame_pool.FrameArrived(&frame_handler).map_err(|_| StreamCreateError::Other("Failed to listen to FrameArrived event".into()))?;
        graphics_capture_item.Closed(&close_handler).map_err(|_| StreamCreateError::Other("Failed to listen to Closed event".into()))?;

        let capture_session = frame_pool.CreateCaptureSession(&graphics_capture_item)
            .map_err(|_| StreamCreateError::Other("Failed to create GraphicsCaptureSession".into()))?;

        capture_session.StartCapture().map_err(|_| StreamCreateError::Other("Failed to start capture".into()))?;

        let stream = WindowsCaptureStream {
            dxgi_adapter,
            dxgi_device,
            d3d11_device,
            frame_pool,
            capture_session,
            should_couninit,
            shared_handler_data
        };

        Ok(stream)
    }

    pub fn stop(&self) -> Result<(), StreamStopError> {
        let already_closed = self.shared_handler_data.closed.fetch_and(true, atomic::Ordering::AcqRel);
        if !already_closed {
            (*self.shared_handler_data.callback.lock())(Ok(StreamEvent::End));
        }
        self.capture_session.Close().map_err(|_| StreamStopError::Other("Failed to close capture session".into()))?;
        Ok(())
    }
}

impl Drop for WindowsCaptureStream {
    fn drop(&mut self) {
        let _ = self.stop();
        if self.should_couninit {
            unsafe { CoUninitialize(); }
        }
    }
}
