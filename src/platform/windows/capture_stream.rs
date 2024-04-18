use std::{sync::{atomic::{self, AtomicBool, AtomicU64}, Arc}, time::{Duration, Instant}, fmt::Debug};

use crate::{prelude::{AudioChannelCount, AudioFrame, Capturable, CaptureConfig, CapturePixelFormat, StreamCreateError, StreamError, StreamEvent, StreamStopError, VideoFrame}, util::Rect};

use parking_lot::Mutex;
use windows::{core::{ComInterface, IInspectable, HSTRING}, Foundation::TypedEventHandler, Graphics::{Capture::{Direct3D11CaptureFramePool, GraphicsCaptureAccess, GraphicsCaptureAccessKind, GraphicsCaptureItem, GraphicsCaptureSession}, DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat}, SizeInt32}, Security::Authorization::AppCapabilityAccess::{AppCapability, AppCapabilityAccessStatus}, Win32::{Graphics::{Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_UNKNOWN, D3D_FEATURE_LEVEL_11_0}, Direct3D11::{D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION}, Dxgi::{CreateDXGIFactory, IDXGIAdapter, IDXGIDevice, IDXGIFactory}}, System::{Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED, COINIT_MULTITHREADED}, WinRT::{Direct3D11::CreateDirect3D11DeviceFromDXGIDevice, Graphics::Capture::IGraphicsCaptureItemInterop}}, UI::HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_RAW_DPI}}};

use super::{audio_capture_stream::{WindowsAudioCaptureStream, WindowsAudioCaptureStreamError, WindowsAudioCaptureStreamPacket}, frame::WindowsVideoFrame, frame::WindowsAudioFrame};

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

#[derive(Clone)]
pub struct WindowsCaptureConfig {
    pub(crate) dxgi_adapter: Option<IDXGIAdapter>,
    pub(crate) d3d11_device: Option<ID3D11Device>,
    #[cfg(feature = "wgpu")]
    pub(crate) wgpu_device: Option<Arc<dyn AsRef<wgpu::Device> + Send + Sync + 'static>>,
}

impl Debug for WindowsCaptureConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowsCaptureConfig").field("dxgi_adapter", &self.dxgi_adapter).field("d3d11_device", &self.d3d11_device).finish()
    }
}

impl WindowsCaptureConfig {
    pub fn new() -> Self {
        Self {
            dxgi_adapter: None,
            d3d11_device: None,
            #[cfg(feature = "wgpu")]
            wgpu_device: None,
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
    pub(crate) dxgi_adapter: Option<IDXGIAdapter>,
    pub(crate) dxgi_adapter_error: Option<String>,
    pub(crate) dxgi_device: IDXGIDevice,
    pub(crate) d3d11_device: ID3D11Device,
    pub(crate) frame_pool: Direct3D11CaptureFramePool,
    pub(crate) capture_session: GraphicsCaptureSession,
    should_couninit: bool,
    shared_handler_data: Arc<SharedHandlerData>,
    audio_stream: Option<WindowsAudioCaptureStream>,
}

pub(crate) struct SharedHandlerData {
    callback: Mutex<Box<dyn FnMut(Result<StreamEvent, StreamError>) + Send + 'static>>,
    closed: AtomicBool,
    frame_id_counter: AtomicU64,
    audio_frame_id_counter: AtomicU64,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct WindowsCaptureAccessToken {
    borderless: bool,
}

impl WindowsCaptureAccessToken {
    pub(crate) fn allows_borderless(&self) -> bool {
        self.borderless
    }
}

impl WindowsCaptureStream {
    pub fn supported_pixel_formats() -> &'static [CapturePixelFormat] {
        &[
            CapturePixelFormat::Bgra8888,
            CapturePixelFormat::Argb2101010,
        ]
    }

    pub fn check_access(borderless: bool) -> Option<WindowsCaptureAccessToken> {
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
        if programmatic_access && (!borderless || borderless_access) {
            Some(WindowsCaptureAccessToken { borderless: borderless || borderless_access })
        } else {
            None
        }
    }

    pub async fn request_access(borderless: bool) -> Option<WindowsCaptureAccessToken> {
        let access_kind = if borderless {
            GraphicsCaptureAccessKind::Borderless
        } else {
            GraphicsCaptureAccessKind::Programmatic
        };
        match GraphicsCaptureAccess::RequestAccessAsync(access_kind) {
            Ok(access_future) => match access_future.await {
                Ok(AppCapabilityAccessStatus::Allowed) => Some(WindowsCaptureAccessToken { borderless }),
                _ => None
            },
            _ => None,
        }
    }

    fn create_d3d11_device(dxgi_adapter: IDXGIAdapter) -> Result<(Option<IDXGIAdapter>, Option<String>, ID3D11Device), StreamCreateError> {
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
                Ok(_) => d3d11_device.map_or_else(|| Err(StreamCreateError::Other("Failed to create ID3D11Device".into())), |x| Ok((Some(dxgi_adapter), None, x))),
                Err(e) => Err(StreamCreateError::Other(format!("Failed to create d3d11 device")))
                ,
            }
        }
    }

    pub fn new(token: WindowsCaptureAccessToken, config: CaptureConfig, callback: Box<impl FnMut(Result<StreamEvent, StreamError>) + Send + 'static>) -> Result<Self, StreamCreateError> {
        let _ = token;
        let should_couninit = unsafe {
            CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok()
        };
        
        let pixel_format = match config.pixel_format {
            CapturePixelFormat::Bgra8888 => DirectXPixelFormat::B8G8R8A8UIntNormalized,
            CapturePixelFormat::Argb2101010 => DirectXPixelFormat::R10G10B10A2UIntNormalized,
            _ => return Err(StreamCreateError::UnsupportedPixelFormat),
        };

        let interop: IGraphicsCaptureItemInterop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()
            .map_err(|_| StreamCreateError::Other("Failed to create IGraphicsCaptureInterop factory".into()))?;

        let callback_target = config.target.clone();

        let graphics_capture_item: GraphicsCaptureItem = unsafe {
            match config.target {
                Capturable::Window(window) =>
                    interop.CreateForWindow(window.impl_capturable_window.0)
                        .map_err(|e| StreamCreateError::Other(format!("Failed to create graphics capture item from HWND: {}", e.to_string())))?,
                Capturable::Display(display) => 
                    interop.CreateForMonitor(display.impl_capturable_display.0)
                        .map_err(|_| StreamCreateError::Other("Failed to create graphics capture item from HMONITOR".into()))?,
            }
        };

        let (dxgi_adapter, dxgi_adapter_error, d3d11_device) = match (config.impl_capture_config.dxgi_adapter, config.impl_capture_config.d3d11_device) {
            (_, Some(d3d11_device)) => {
                let dxgi_adapter = d3d11_device.cast().map_err(|error| format!("Failed to create IDXGIAdapter from ID3D11Device: {}", error.to_string()));
                match dxgi_adapter {
                    Ok(dxgi_adapter) => (Some(dxgi_adapter), None, d3d11_device),
                    Err(dxgi_adapter_error) => (None, Some(dxgi_adapter_error), d3d11_device)
                }
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
                audio_frame_id_counter: AtomicU64::new(0),
            }
        );

        let close_handler_data = shared_handler_data.clone();
        let frame_handler_data = shared_handler_data.clone();
        let audio_handler_data = shared_handler_data.clone();

        let close_handler = TypedEventHandler::new(move |_, _| {
            let alread_closed = close_handler_data.closed.fetch_and(true, atomic::Ordering::AcqRel);
            if !alread_closed {
                let mut callback = close_handler_data.callback.lock();
                (*callback)(Ok(StreamEvent::End));
            }
            Ok(())
        });

        let mut t_first_frame = None;
        let mut t_last_frame = None;

        #[cfg(feature = "wgpu")]
        let callback_wgpu_device = config.impl_capture_config.wgpu_device;

        let frame_handler = TypedEventHandler::new(move |frame_pool: &Option<Direct3D11CaptureFramePool>, _: &Option<IInspectable>| {
            if frame_pool.is_none() {
                return Ok(());
            }
            let frame_pool = frame_pool.as_ref().unwrap();
            if frame_handler_data.closed.load(atomic::Ordering::Acquire) {
                return Ok(());
            }
            let t_capture = Instant::now();
            let t_origin = match t_first_frame {
                Some(t_first_frame) => t_capture - t_first_frame,
                None => {
                    t_first_frame = Some(t_capture);
                    Duration::ZERO
                }
            };
            let duration = match t_last_frame {
                Some(t_last_frame) => t_capture - t_last_frame,
                None => {
                    t_last_frame = Some(t_capture);
                    Duration::ZERO
                }
            };
            let dpi = unsafe { 
                match &callback_target {
                    Capturable::Window(window) => GetDpiForWindow(window.impl_capturable_window.0),
                    Capturable::Display(display) => {
                        let mut dpi_x = 0u32;
                        let mut dpi_y = 0u32;
                        let _ = GetDpiForMonitor(display.impl_capturable_display.0, MDT_RAW_DPI, &mut dpi_x as *mut _, &mut dpi_y as *mut _);
                        dpi_x.min(dpi_y)
                    }
                }
            };
            let mut callback = frame_handler_data.callback.lock();
            //let window_rect = RECT::default();
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
                pixel_format,
                dpi,
                t_capture,
                t_origin,
                duration,
                #[cfg(feature = "wgpu")]
                wgpu_device: callback_wgpu_device.clone()
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

        let audio_stream = if let Some(audio_config) = config.capture_audio {
            let handler_config = audio_config.clone();
            let audio_handler = Box::new(move |audio_result: Result<WindowsAudioCaptureStreamPacket<'_>, WindowsAudioCaptureStreamError>| {
                if audio_handler_data.closed.load(atomic::Ordering::Acquire) {
                    return;
                }
                match audio_result {
                    Ok(packet) => {
                        let audio_frame_id = audio_handler_data.audio_frame_id_counter.fetch_add(1, atomic::Ordering::AcqRel);
                        let event = StreamEvent::Audio(AudioFrame {
                            impl_audio_frame: WindowsAudioFrame {
                                data: packet.data.to_owned().into_boxed_slice(),
                                channel_count: handler_config.channel_count,
                                sample_rate: handler_config.sample_rate,
                                duration: packet.duration,
                                origin_time: packet.origin_time,
                                frame_id: audio_frame_id
                            }
                        });
                        (*audio_handler_data.callback.lock())(Ok(event));
                    },
                    Err(e) => {
                        (*audio_handler_data.callback.lock())(Err(StreamError::Other("Audio stream error".to_string())));
                    }
                }
            });

            match WindowsAudioCaptureStream::new(audio_config, audio_handler) {
                Ok(audio_stream) => {
                    Some(audio_stream)
                },
                Err(_) => {
                    return Err(StreamCreateError::Other("Failed to create audio stream".into()))
                }
            }
        } else {
            None
        };

        capture_session.StartCapture().map_err(|_| StreamCreateError::Other("Failed to start capture".into()))?;

        let stream = WindowsCaptureStream {
            dxgi_adapter,
            dxgi_adapter_error,
            dxgi_device,
            d3d11_device,
            frame_pool,
            capture_session,
            should_couninit,
            shared_handler_data,
            audio_stream
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
        if let Some(audio_stream) = &mut self.audio_stream {
            audio_stream.stop();
        }
        if self.should_couninit {
            unsafe { CoUninitialize(); }
        }
    }
}
