use std::ffi::c_void;

use pipewire::spa::param::video::VideoInfoRaw;

use crate::prelude::VideoCaptureFrame;

pub(crate) struct WaylandVideoFrame {
    pub(crate) size: crate::prelude::Size,
    pub(crate) id: u64,
    pub(crate) captured: std::time::Instant,
    pub(crate) pts: std::time::Duration,
    pub(crate) format: VideoInfoRaw,
    pub(crate) data: *const c_void,
}

impl VideoCaptureFrame for WaylandVideoFrame {
    fn size(&self) -> crate::prelude::Size {
        self.size
    }

    fn dpi(&self) -> f64 {
        todo!()
    }

    fn duration(&self) -> std::time::Duration {
        std::time::Duration::from_secs(0)
    }

    fn origin_time(&self) -> std::time::Duration {
        self.pts
    }

    fn capture_time(&self) -> std::time::Instant {
        self.captured
    }

    fn frame_id(&self) -> u64 {
        self.id
    }

    fn content_rect(&self) -> crate::prelude::Rect {
        todo!()
    }
}
