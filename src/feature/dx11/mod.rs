#![cfg(target_os = "windows")]
#![cfg(feature = "dx11")]

use futures::lock::Mutex;
use windows::{Graphics::DirectX::{Direct3D11::IDirect3DSurface, DirectXPixelFormat}, Win32::Graphics::Direct3D11::ID3D11Device};

use std::error::Error;
use std::fmt::Display;

use crate::prelude::{CaptureStream, VideoFrame};

#[derive(Debug, Clone)]
pub enum WindowsDx11VideoFrameError {
    Other(String),
}

impl Display for WindowsDx11VideoFrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Other(error) => f.write_fmt(format_args!("WindowsDx11VideoFrameError::Other(\"{}\")", error)),
        }
    }
}

impl Error for WindowsDx11VideoFrameError {
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

/// A video frame which can yield a d3d11 surface
pub trait WindowsDx11VideoFrame {
    /// Get the DX11 surface representing the video frame's texture memory, as well as the pixel format
    fn get_dx11_surface(&self) -> Result<(IDirect3DSurface, DirectXPixelFormat), WindowsDx11VideoFrameError>;
}

impl WindowsDx11VideoFrame for VideoFrame {
    fn get_dx11_surface(&self) -> Result<(IDirect3DSurface, DirectXPixelFormat), WindowsDx11VideoFrameError> {
        self.impl_video_frame.frame.Surface()
            .map_err(|e| WindowsDx11VideoFrameError::Other(format!("Failed to get frame surface: {}", e.to_string())))
            .map(|surface| (surface, self.impl_video_frame.pixel_format))
    }
}

/// A capture stream which can interoperate with D3D11
pub trait WindowsDx11CaptureStream {
    /// Get the underlying D3D11 device used for frame capture
    fn get_dx11_device(&self) -> ID3D11Device;
}

impl WindowsDx11CaptureStream for CaptureStream {
    fn get_dx11_device(&self) -> ID3D11Device {
        self.impl_capture_stream.d3d11_device.clone()
    }
}
