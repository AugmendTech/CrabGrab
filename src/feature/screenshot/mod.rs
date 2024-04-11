mod platform;

use super::bitmap::*;

use crate::capturable_content::CapturableWindow;

pub enum ScreenshotError {
    Other(String)
}

pub fn take_window_screenshot(window: CapturableWindow) -> Result<FrameBitmap, ScreenshotError> {
    Err(ScreenshotError::Other("Unimplemented".into()))
}
