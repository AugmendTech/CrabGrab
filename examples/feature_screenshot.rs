use crabgrab::{feature::bitmap::VideoFrameBitmap, prelude::*};
use futures::executor::block_on;
 
fn main() { 
    block_on(async {
        let token = match CaptureStream::test_access(false) {
            Some(token) => token,
            None => CaptureStream::request_access(false).await.expect("Expected capture access")
        };
        let window_filter = CapturableWindowFilter {
            desktop_windows: false,
            onscreen_only: true,
        };
        let filter = CapturableContentFilter { windows: Some(window_filter), displays: false };
        let content = CapturableContent::new(filter).await.unwrap();
        let window = content.windows().filter(|window| {
            let app_identifier = window.application().identifier();
            window.title().len() != 0 && app_identifier.to_lowercase().contains("firefox")
        }).next();
        match window {
            Some(window) => {
                println!("screenshotting window: {}", window.title()); 
                let config = CaptureConfig::with_window(window, CaptureStream::supported_pixel_formats()[0]).unwrap();
                match crabgrab::feature::screenshot::take_screenshot(token, config).await {
                    Ok(frame) => { 
                        println!("Got frame: {}", frame.frame_id());
                        match frame.get_bitmap() {
                            Ok(bitmap) => {
                                match bitmap {
                                    crabgrab::feature::bitmap::FrameBitmap::BgraUnorm8x4(_) => println!("format: BgraUnorm8x4"),
                                    crabgrab::feature::bitmap::FrameBitmap::RgbaUnormPacked1010102(_) => println!("format: RgbaUnormPacked1010102"),
                                    crabgrab::feature::bitmap::FrameBitmap::RgbaF16x4(_) => println!("format: RgbaF16x4"),
                                    crabgrab::feature::bitmap::FrameBitmap::YCbCr(_) => println!("format: YCbCr"),
                                }
                            },
                            Err(e) => {
                                println!("Bitmap error: {:?}", e);
                            }
                        }
                    },
                    Err(_) => println!("screenshot failed!"),
                }
            },
            None => { println!("Failed to find window"); }
        }
    });
}
