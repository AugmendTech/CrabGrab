use std::time::Duration;

use crabgrab::prelude::*;

fn main() { 
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .build().unwrap();
    let future = runtime.spawn(async {
        if !CaptureStream::test_access(false) {
            CaptureStream::request_access(false).await;
            println!("Approve access and run again!");
        }
        let window_filter = CapturableWindowFilter {
            desktop_windows: false,
            onscreen_only: true,
        };
        let filter = CapturableContentFilter { windows: Some(window_filter), displays: false };
        let content = CapturableContent::new(filter).await.unwrap();
        let window = content.windows().filter(|window| {
            let app_identifier = window.application().identifier();
            app_identifier.to_lowercase().contains("finder") || app_identifier.to_lowercase().contains("explorer")
        }).next();
        match window {
            Some(window) => {
                println!("capturing window: {}", window.title()); 
                let config = CaptureConfig::with_window(window, CaptureStream::supported_pixel_formats()[3]).unwrap();
                let mut stream = CaptureStream::new(config, |stream_event| {
                    match stream_event {
                        Ok(event) => {
                            match event {
                                StreamEvent::Video(frame) => {
                                    println!("Got frame: {}", frame.frame_id());
                                },
                                _ => {}
                            }
                        },
                        Err(error) => {
                            println!("Stream error: {:?}", error);
                        }
                    }
                }).unwrap();
                println!("stream created!"); 
                tokio::task::block_in_place(|| std::thread::sleep(Duration::from_millis(4000)));
                stream.stop().unwrap();
            },
            None => { println!("Failed to find window"); }
        }
    });
    runtime.block_on(future).unwrap();
    runtime.shutdown_timeout(Duration::from_millis(10000));
}
