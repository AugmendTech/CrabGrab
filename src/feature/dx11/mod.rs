use futures::lock::Mutex;
use windows::{Graphics::DirectX::Direct3D11::IDirect3DSurface, Win32::Graphics::Direct3D11::ID3D11Device};

use crate::prelude::{CaptureStream, VideoFrame};

#[derive(Debug, Clone)]
pub enum WindowsDx11VideoFrameError {
    Other(String),
}

pub trait WindowsDx11VideoFrame {
    fn get_dx11_surface(&self) -> Result<IDirect3DSurface, WindowsDx11VideoFrameError>;
}

impl WindowsDx11VideoFrame for VideoFrame {
    /// Get the surface texture for this video frame
    fn get_dx11_surface(&self) -> Result<IDirect3DSurface, WindowsDx11VideoFrameError> {
        self.impl_video_frame.frame.Surface()
            .map_err(|e| WindowsDxgiVideoFrameError::Other(format!("Failed to get frame surface: {}", e.to_string())))
    }
}

pub trait WindowsDx11CaptureStream {
    fn get_dx11_device(&self) -> ID3D11Device;
}

impl WindowsDx11CaptureStream for CaptureStream {
    fn get_dx11_device(&self) -> ID3D11Device {
        self.impl_capture_stream.d3d11_device.clone()
    }
}
