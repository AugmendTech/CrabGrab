#![cfg(target_os = "macos")]
#![cfg(feature = "iosurface")]

use crate::prelude::VideoFrame;

/// A Macos IOSurface instance
pub struct IoSurface {
    raw: *mut objc::runtime::Object,
}

impl IoSurface {
    /// Gets the raw "id" object pointer for the IOSurface instance
    pub fn get_raw(&self) -> *mut objc::runtime::Object {
        self.raw
    }
}

pub trait MacosIoSurfaceVideoFrame {
    /// Get the iosurface of the video frame
    fn get_iosurface(&self) -> IoSurface;
}

impl MacosIoSurfaceVideoFrame for VideoFrame {
    fn get_iosurface(&self) -> IoSurface {
        todo!()
    }
}
