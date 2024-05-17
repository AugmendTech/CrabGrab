use std::time::Duration;

use crabgrab::{platform::macos::MacosCapturableWindowExt, prelude::*};

fn main() { 
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .build().unwrap();
    let future = runtime.spawn(async {
        let token = match CaptureStream::test_access(true) {
            Some(token) => token,
            None => CaptureStream::request_access(true).await.expect("Expected capture access")
        };
        let filter = CapturableContentFilter::NORMAL_WINDOWS;
        let content = CapturableContent::new(filter).await.unwrap();
        for window in content.windows() {
            println!("window: {}, app: {}, window layer: {:?}, window level: {:?}", window.title(), window.application().identifier(), window.get_window_layer().ok(), window.get_window_level().ok());
        }
    });
    runtime.block_on(future).unwrap();
    runtime.shutdown_timeout(Duration::from_millis(10000));
}
