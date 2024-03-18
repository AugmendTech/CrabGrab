#![cfg(target_os = "macos")]
#![cfg(feature = "iosurface")]

use std::os::raw::c_void;

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

pub enum GetIoSurfaceError{
    NoImageBuffer,
    NoIoSurface
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
