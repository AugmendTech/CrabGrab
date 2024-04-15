use crate::feature::screenshot::ScreenshotError;
use crate::capturable_content::CapturableWindow;
use crate::frame::VideoFrame;

pub async fn take_screenshot(config: CaptureConfig) -> Result<VideoFrame, ScreenshotError> {
    Err(ScreenshotError::Other("Unimplemented".into()))
}