use std::time::Duration;

use crabgrab::{feature::diagnostic::FrameDiagnosticExt, prelude::*};

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
        let window = content.windows().filter(|window| {
            let app_identifier = window.application().identifier();
            window.title().len() != 0 && (app_identifier.to_lowercase().contains("terminal") || app_identifier.to_lowercase().contains("explorer"))
        }).next();
        match window {
            Some(window) => {
                let config = CaptureConfig::with_window(window, CapturePixelFormat::Bgra8888).unwrap();
                let mut diag_done = false;
                let mut stream = CaptureStream::new(token, config, move |stream_event| {
                    match stream_event {
                        Ok(event) => {
                            match event {
                                StreamEvent::Video(frame) => {
                                    if !diag_done {
                                        println!("Frame diagnostic: {:?}", frame.diagnostic());
                                        diag_done = true;
                                    }
                                },
                                _ => {}
                            }
                        },
                        Err(error) => {
                            println!("Stream error: {:?}", error);
                        }
                    }
                }).unwrap();
                tokio::task::block_in_place(|| std::thread::sleep(Duration::from_millis(2000)));
                stream.stop().unwrap();
            },
            None => { println!("Failed to find window"); }
        }
    });
    runtime.block_on(future).unwrap();
    runtime.shutdown_timeout(Duration::from_millis(100000));
}
