use futures::channel::oneshot;

use crate::feature::screenshot::ScreenshotError;
use crate::frame::VideoFrame;
use crate::prelude::{CaptureConfig, CaptureStream, StreamEvent};

pub async fn take_screenshot(config: CaptureConfig) -> Result<VideoFrame, ScreenshotError> {
    let (tx, rx) = oneshot::channel();
    let mut tx = Some(tx);
    let mut capture_stream = CaptureStream::new(config, move |event_result| {
        match event_result {
            Ok(StreamEvent::Video(frame)) => {
                if let Some(tx) = tx.take() {
                    let _ = tx.send(Ok(frame));
                }
            },
            Err(e) => {
                if let Some(tx) = tx.take() {
                    let _ = tx.send(Err(e));
                }
            },
            _ => {}
        }
    }).map_err(|error| {
        ScreenshotError::Other(format!("Failed to create capture stream: {}", error.to_string()))
    })?;
    let result = rx.await.map_err(|_| ScreenshotError::Other("Failed to wait for result from callback".into()))?;
    let _ = capture_stream.stop();
    result.map_err(|error| ScreenshotError::Other(format!("Capture failed: {}", error.to_string())))
}