#![cfg(target_os = "windows")]
#![cfg(feature = "dxgi")]

use crate::prelude::{CaptureStream, VideoFrame};

use std::error::Error;
use std::fmt::Display;

use windows::core::ComInterface;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;

#[derive(Debug, Clone)]
pub enum WindowsDxgiVideoFrameError {
    Other(String),
}

impl Display for WindowsDxgiVideoFrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(error) => f.write_fmt(format_args!("WindowsDxgiVideoFrameError::Other(\"{}\")", error)),
        }
    }
}

impl Error for WindowsDxgiVideoFrameError {
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

/// A video frame which can inter-operate with DXGI
pub trait WindowsDxgiVideoFrame {
    /// Get the surface texture for this video frame
    fn get_dxgi_surface(&self) -> Result<(windows::Win32::Graphics::Dxgi::IDXGISurface, DirectXPixelFormat), WindowsDxgiVideoFrameError>; 
}

impl WindowsDxgiVideoFrame for VideoFrame {
    fn get_dxgi_surface(&self) -> Result<(windows::Win32::Graphics::Dxgi::IDXGISurface, DirectXPixelFormat), WindowsDxgiVideoFrameError> {
        let d3d11_surface = self.impl_video_frame.frame.Surface()
            .map_err(|e| WindowsDxgiVideoFrameError::Other(format!("Failed to get frame surface: {}", e.to_string())))?;
        let interface_access: IDirect3DDxgiInterfaceAccess = d3d11_surface.cast()
            .map_err(|e| WindowsDxgiVideoFrameError::Other(format!("Failed to cast d3d11 surface to dxgi interface access: {}", e.to_string())))?;
        let d3d11_texture: ID3D11Texture2D = unsafe {
            interface_access.GetInterface::<ID3D11Texture2D>()
        }.map_err(|e| WindowsDxgiVideoFrameError::Other(format!("Failed to get ID3D11Texture2D interface from to IDirect3DSurface(IDirect3DDxgiInterfaceAccess): {}", e.to_string())))?;
        d3d11_texture.cast().map_err(|e| WindowsDxgiVideoFrameError::Other(format!("Failed to cast ID3D11Texture2D to IDXGISurface: {}", e.to_string())))
            .map(|texture| (texture, self.impl_video_frame.pixel_format))
    }
}

#[derive(Debug)]
pub enum WindowsDxgiCaptureStreamError {
    NoAdapter(String)
}

impl Display for WindowsDxgiCaptureStreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoAdapter(error) => f.write_fmt(format_args!("WindowsDxgiCaptureStreamError::NoAdapter(\"{}\")", error)),
        }
    }
}

impl Error for WindowsDxgiCaptureStreamError {
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

/// A capture stream which can inter-operate with DXGI
pub trait WindowsDxgiCaptureStream {
    /// Get the DXGI adapter used by the capture stream for frame generation
    fn get_dxgi_adapter(&self) -> Result<windows::Win32::Graphics::Dxgi::IDXGIAdapter, WindowsDxgiCaptureStreamError>;
    /// Get the DXGI device used by the capture stream for frame generation
    fn get_dxgi_device(&self) -> windows::Win32::Graphics::Dxgi::IDXGIDevice;
}

impl WindowsDxgiCaptureStream for CaptureStream {
    fn get_dxgi_adapter(&self) -> Result<windows::Win32::Graphics::Dxgi::IDXGIAdapter, WindowsDxgiCaptureStreamError> {
        if let Some(dxgi_adapter) = self.impl_capture_stream.dxgi_adapter.clone() {
            Ok(dxgi_adapter)
        } else {
            match &self.impl_capture_stream.dxgi_adapter_error {
                Some(error) => Err(WindowsDxgiCaptureStreamError::NoAdapter(error.clone())),
                None => unreachable!("Should have dxgi_adapter_error if dxgi_adapter is None")
            }
        }
    }

    fn get_dxgi_device(&self) -> windows::Win32::Graphics::Dxgi::IDXGIDevice {
        self.impl_capture_stream.dxgi_device.clone()
    }
}
