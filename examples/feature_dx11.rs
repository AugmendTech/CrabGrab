use std::time::Duration;

use crabgrab::prelude::*;

#[tokio::main]
async fn main() { 
    let token = match CaptureStream::test_access(false) {
        Some(token) => token,
        None => CaptureStream::request_access(false).await.expect("Expected capture access")
    };
    let filter = CapturableContentFilter::DISPLAYS;
    let content = CapturableContent::new(filter).await.unwrap();
    let config = CaptureConfig::with_display(content.displays().next().unwrap(), CapturePixelFormat::Bgra8888);

    let mut stream = CaptureStream::new(token, config, |result| {
        if let StreamEvent::Video(frame) = result.expect("Expected stream event") {
            if let Ok(_dx11_texture) = frame.get_dx11_texture() {
                println!("Got dx11 texture: {}", frame.frame_id());
            } else {
                println!("Failed to get dx11 texture");
            }
        }
    }).unwrap();

    std::thread::sleep(Duration::from_millis(20000));

    stream.stop().unwrap();
}
