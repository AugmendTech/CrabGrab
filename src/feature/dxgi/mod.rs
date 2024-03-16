#![cfg(target_os = "windows")]
#![cfg(feature = "dxgi")]

use crate::prelude::{CaptureStream, VideoFrame};

use windows::core::ComInterface;
use windows::Win32::System::WinRT::Direct3D11::IDirect3DDxgiInterfaceAccess;
use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;

#[derive(Debug, Clone)]
pub enum WindowsDxgiVideoFrameError {
    Other(String),
}

pub trait WindowsDxgiVideoFrame {
    /// Get the surface texture for this video frame
    fn get_dxgi_surface(&self) -> Result<windows::Win32::Graphics::Dxgi::IDXGISurface, WindowsDxgiVideoFrameError>; 
}

impl WindowsDxgiVideoFrame for VideoFrame {
    fn get_dxgi_surface(&self) -> Result<windows::Win32::Graphics::Dxgi::IDXGISurface, WindowsDxgiVideoFrameError> {
        let d3d11_surface = self.impl_video_frame.frame.Surface()
            .map_err(|e| WindowsDxgiVideoFrameError::Other(format!("Failed to get frame surface: {}", e.to_string())))?;
        let interface_access: IDirect3DDxgiInterfaceAccess = d3d11_surface.cast()
            .map_err(|e| WindowsDxgiVideoFrameError::Other(format!("Failed to cast d3d11 surface to dxgi interface access: {}", e.to_string())))?;
        let d3d11_texture: ID3D11Texture2D = unsafe {
            interface_access.GetInterface::<ID3D11Texture2D>()
        }.map_err(|e| WindowsDxgiVideoFrameError::Other(format!("Failed to get ID3D11Texture2D interface from to IDirect3DSurface(IDirect3DDxgiInterfaceAccess): {}", e.to_string())))?;
        d3d11_texture.cast().map_err(|e| WindowsDxgiVideoFrameError::Other(format!("Failed to cast ID3D11Texture2D to IDXGISurface: {}", e.to_string())))
    }
}

pub trait WindowsDxgiCaptureStream {
    /// Get the dxgi adapter used by the capture stream for frame generation
    fn get_dxgi_adapter(&self) -> windows::Win32::Graphics::Dxgi::IDXGIAdapter;
    /// Get the dxgi device used by the capture stream for frame generation
    fn get_dxgi_device(&self) -> windows::Win32::Graphics::Dxgi::IDXGIDevice;
}

impl WindowsDxgiCaptureStream for CaptureStream {
    fn get_dxgi_adapter(&self) -> windows::Win32::Graphics::Dxgi::IDXGIAdapter {
        self.impl_capture_stream.dxgi_adapter.clone()
    }

    fn get_dxgi_device(&self) -> windows::Win32::Graphics::Dxgi::IDXGIDevice {
        self.impl_capture_stream.dxgi_device.clone()
    }
}
