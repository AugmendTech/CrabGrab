use std::sync::Arc;

use crabgrab::prelude::*;
use crabgrab::util::Size;
use futures::channel::{mpsc, oneshot};
use futures::{SinkExt, StreamExt};
use parking_lot::Mutex;

#[tokio::test]
async fn capture_display() {
    if !CaptureStream::test_access(false) {
        let has_permission = CaptureStream::request_access(false).await;
        assert!(has_permission);
    }
    let content_filter = CapturableContentFilter {
        windows: None,
        displays: true
    };
    let content = CapturableContent::new(content_filter).await;
    assert!(content.is_ok());
    let content = content.unwrap();
    let mut displays = content.displays();
    println!("display count: {}", displays.len());
    let display_opt = displays.next();
    assert!(display_opt.is_some());
    let display = display_opt.unwrap();
    println!("display: {:?}", display.rect());
    //let size = display.rect().size;
    let config = CaptureConfig::with_display(display, CapturePixelFormat::Bgra8888);
    let (tx, rx) = oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));
    let new_stream_result = CaptureStream::new(config, move |result| {
        println!("stream result: {:?}", result);
        if let Some(tx) = tx.lock().take() {
            tx.send(result.is_ok()).unwrap();
        }
    });
    if let Err(e) = &new_stream_result {
        println!("Stream create error: {:?}", e);
    } else {
        println!("Stream created!");
    }
    assert!(new_stream_result.is_ok());
    let stream_callback_result = rx.await.unwrap();
    assert!(stream_callback_result);
}
