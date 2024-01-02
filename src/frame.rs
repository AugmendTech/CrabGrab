use std::time::{Duration, Instant};

use crate::util::*;

pub trait VideoCaptureFrame {
    fn logical_frame(&self) -> Rect;
    fn physical_frame(&self) -> Rect;
    fn content_scale(&self) -> Rect;
    fn duration(&self) -> Duration;
    fn origin_time(&self) -> Instant;
    fn capture_time(&self) -> Instant;
    fn frame_id(&self) -> u64;
}

pub trait AudioCaptureFrame {
    fn duration(&self) -> Duration;
    fn origin_time(&self) -> Instant;
    fn capture_time(&self) -> Instant;
    fn frame_id(&self) -> u64;
}
