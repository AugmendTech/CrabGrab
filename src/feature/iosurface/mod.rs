#![cfg(target_os = "macos")]
#![cfg(feature = "iosurface")]

use std::os::raw::c_void;

use std::error::Error;
use std::fmt::Display;

use crate::{platform::{macos::{frame::MacosVideoFrame, objc_wrap::IOSurfaceRef}, platform_impl::objc_wrap::{IOSurfaceDecrementUseCount, IOSurfaceIncrementUseCount}}, prelude::VideoFrame};

/// A Macos IOSurface instance
pub struct IoSurface(IOSurfaceRef);

impl IoSurface {
    /// Gets the raw IOSurfaceRef
    pub fn get_raw(&self) -> *const c_void {
        self.0
    }

    pub(crate) fn from_ref_unretained(r: IOSurfaceRef) -> Self {
        unsafe { IOSurfaceIncrementUseCount(r); }
        IoSurface(r)
    }
}

impl Clone for IoSurface {
    fn clone(&self) -> Self {
        unsafe { IOSurfaceIncrementUseCount(self.0); }
        IoSurface(self.0)
    }
}

impl Drop for IoSurface {
    fn drop(&mut self) {
        unsafe { IOSurfaceDecrementUseCount(self.0); }
    }
}

pub trait MacosIoSurfaceVideoFrame {
    /// Get the iosurface of the video frame
    fn get_iosurface(&self) -> Result<IoSurface, GetIoSurfaceError>;
}

#[derive(Debug)]
pub enum GetIoSurfaceError{
    NoImageBuffer,
    NoIoSurface
}

impl Display for GetIoSurfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoImageBuffer => f.write_str("GetIoSurfaceError::NoImageBuffer"),
            Self::NoIoSurface => f.write_str("GetIoSurfaceError::NoIoSurface"),
        }
    }
}

impl Error for GetIoSurfaceError {
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

impl MacosIoSurfaceVideoFrame for VideoFrame {
    fn get_iosurface(&self) -> Result<IoSurface, GetIoSurfaceError> {
        match &self.impl_video_frame {
            MacosVideoFrame::SCStream(frame) => {
                match frame.sample_buffer.get_image_buffer() {
                    Some(image_buffer) => {
                        match image_buffer.get_iosurface_ptr() {
                            Some(ptr) => {
                                Ok(IoSurface::from_ref_unretained(ptr))
                            },
                            None => Err(GetIoSurfaceError::NoIoSurface)
                        }
                    },
                    None => Err(GetIoSurfaceError::NoImageBuffer)
                }
            },
            MacosVideoFrame::CGDisplayStream(frame) => {
                Ok(IoSurface::from_ref_unretained(frame.io_surface.0))
            }
        }
    }
}
