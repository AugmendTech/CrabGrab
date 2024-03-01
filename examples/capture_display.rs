use std::time::Duration;

use crabgrab::prelude::*;

#[tokio::main]
async fn main() { 
    if !CaptureStream::test_access(false) {
        if !CaptureStream::request_access(false).await {
            println!("Failed to get access!");
            return;
        };
    }
    let filter = CapturableContentFilter { windows: None, displays: true };
    let content = CapturableContent::new(filter).await.unwrap();
    let config = CaptureConfig::with_display(content.displays().next().unwrap(), CapturePixelFormat::Bgra8888);

    let mut stream = CaptureStream::new(config, |result| {
        println!("result: {:?}", result);
    }).unwrap();

    std::thread::sleep(Duration::from_millis(2000));

    stream.stop().unwrap();
}
